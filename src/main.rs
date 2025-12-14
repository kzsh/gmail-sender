use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::Input;
use google_gmail1::{
    api::Message,
    hyper::{
        self, client::HttpConnector, Body, Method, Request,
        header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    },
    hyper_rustls::{self, HttpsConnector},
    oauth2,
};
use std::fs;
use std::path::PathBuf;

/// A simple CLI tool to send emails via Gmail API
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Recipient email address
    #[arg(short, long)]
    to: Option<String>,

    /// Email subject
    #[arg(short, long)]
    subject: Option<String>,

    /// Email body (plain text)
    #[arg(short, long)]
    body: Option<String>,

    /// File attachments (can be specified multiple times)
    #[arg(short, long)]
    attachment: Vec<PathBuf>,

    /// Path to OAuth2 client secret JSON file
    #[arg(long, default_value = "client_secret.json")]
    client_secret: PathBuf,

    /// Path to store OAuth2 tokens
    #[arg(long, default_value = "token_cache.json")]
    token_cache: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Get email details (from args or prompt)
    let to = get_or_prompt(args.to, "To")?;
    let subject = get_or_prompt(args.subject, "Subject")?;
    let body = get_or_prompt(args.body, "Body")?;

    println!("Authenticating with Gmail...");
    let (client, auth) = create_gmail_client(&args.client_secret, &args.token_cache).await?;

    println!("Building email...");
    let email_message = build_email(&to, &subject, &body, &args.attachment)?;

    println!("Sending email...");
    let result = send_email(&client, &auth, email_message).await?;

    println!("✓ Email sent successfully!");
    println!("  Message ID: {}", result.id.unwrap_or_default());
    println!("  Thread ID: {}", result.thread_id.unwrap_or_default());

    Ok(())
}

/// Get value from Option or prompt user interactively
fn get_or_prompt(value: Option<String>, field_name: &str) -> Result<String> {
    match value {
        Some(v) => Ok(v),
        None => Input::new()
            .with_prompt(field_name)
            .interact_text()
            .context(format!("Failed to read {}", field_name)),
    }
}

/// Create authenticated HTTP client and authenticator
async fn create_gmail_client(
    client_secret_path: &PathBuf,
    token_cache_path: &PathBuf,
) -> Result<(hyper::Client<HttpsConnector<HttpConnector>>, oauth2::authenticator::Authenticator<HttpsConnector<HttpConnector>>)> {
    // Read client secret
    let secret = oauth2::read_application_secret(client_secret_path)
        .await
        .context("Failed to read client secret file. Did you download it from Google Cloud Console?")?;

    // Create authenticator with token persistence
    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_cache_path)
    .build()
    .await
    .context("Failed to create authenticator")?;

    // Create HTTP client
    let client = hyper::Client::builder().build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .context("Failed to load native root certificates")?
            .https_or_http()
            .enable_http1()
            .build(),
    );

    Ok((client, auth))
}

/// Send email via direct HTTP POST to Gmail API
async fn send_email(
    client: &hyper::Client<HttpsConnector<HttpConnector>>,
    auth: &oauth2::authenticator::Authenticator<HttpsConnector<HttpConnector>>,
    message: Message,
) -> Result<Message> {
    // Get access token
    let token = auth
        .token(&["https://www.googleapis.com/auth/gmail.send"])
        .await
        .context("Failed to get access token")?;

    // Serialize message to JSON
    let json_body = serde_json::to_string(&message)
        .context("Failed to serialize message to JSON")?;

    // Build HTTP request
    let req = Request::builder()
        .method(Method::POST)
        .uri("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
        .header(AUTHORIZATION, format!("Bearer {}", token.token().unwrap()))
        .header(CONTENT_TYPE, "application/json")
        .header(USER_AGENT, "gmail-sender/0.1.0")
        .body(Body::from(json_body))
        .context("Failed to build HTTP request")?;

    // Send request
    let resp = client.request(req).await
        .context("Failed to send HTTP request")?;

    // Check response status
    if !resp.status().is_success() {
        let status = resp.status();
        let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
        let body_str = String::from_utf8_lossy(&body_bytes);
        anyhow::bail!("Gmail API request failed with status {}: {}", status, body_str);
    }

    // Parse response
    let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
    let result: Message = serde_json::from_slice(&body_bytes)
        .context("Failed to parse response JSON")?;

    Ok(result)
}

/// Build email message with optional attachments
fn build_email(
    to: &str,
    subject: &str,
    body: &str,
    attachments: &[PathBuf],
) -> Result<Message> {
    let boundary = "boundary_xyz123";

    // Build headers - RFC822 requires CRLF line endings
    let mut headers = vec![
        format!("From: me"),  // Gmail API will replace 'me' with authenticated user
        format!("To: {}", to),
        format!("Subject: {}", subject),
        "MIME-Version: 1.0".to_string(),
    ];

    // Build body
    let mut email_body = String::new();

    if attachments.is_empty() {
        // Simple plain text email
        headers.push("Content-Type: text/plain; charset=UTF-8".to_string());
        email_body = body.to_string();
    } else {
        // Multipart email with attachments
        headers.push(format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"",
            boundary
        ));

        // Text part
        email_body.push_str(&format!("--{}\r\n", boundary));
        email_body.push_str("Content-Type: text/plain; charset=UTF-8\r\n\r\n");
        email_body.push_str(body);
        email_body.push_str("\r\n\r\n");

        // Attachment parts
        for attachment_path in attachments {
            let filename = attachment_path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Invalid attachment filename")?;

            let file_data = fs::read(attachment_path)
                .context(format!("Failed to read attachment: {:?}", attachment_path))?;

            let mime_type = mime_guess::from_path(attachment_path)
                .first_or_octet_stream()
                .to_string();

            let encoded_data = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &file_data,
            );

            email_body.push_str(&format!("--{}\r\n", boundary));
            email_body.push_str(&format!("Content-Type: {}\r\n", mime_type));
            email_body.push_str(&format!(
                "Content-Disposition: attachment; filename=\"{}\"\r\n",
                filename
            ));
            email_body.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            email_body.push_str(&encoded_data);
            email_body.push_str("\r\n\r\n");
        }

        email_body.push_str(&format!("--{}--", boundary));
    }

    // Combine headers and body with CRLF line endings (RFC822)
    let raw_email = format!("{}\r\n\r\n{}", headers.join("\r\n"), email_body);

    // Debug: print the raw email
    eprintln!("DEBUG - Raw email:");
    eprintln!("{}", raw_email.replace("\r\n", "\\r\\n\n"));
    eprintln!("---");

    // IMPORTANT: Don't base64 encode ourselves!
    // The Message struct's `raw` field has a serde attribute that will automatically
    // base64url-encode the Vec<u8> during JSON serialization
    Ok(Message {
        raw: Some(raw_email.into_bytes()),
        ..Default::default()
    })
}
