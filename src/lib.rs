//! A library for sending emails via the Gmail API.
//!
//! This crate provides functions for OAuth2 authentication with Gmail and
//! building/sending RFC822-compliant email messages.
//!
//! # Example
//!
//! ```no_run
//! use gmail_sender::{create_gmail_client, build_email, send_email};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client_secret = PathBuf::from("client_secret.json");
//!     let token_cache = PathBuf::from("token_cache.json");
//!
//!     let (client, auth) = create_gmail_client(&client_secret, &token_cache).await?;
//!
//!     let message = build_email("user@example.com", "Hello", "Email body", &[], false)?;
//!     let result = send_email(&client, &auth, message).await?;
//!
//!     println!("Sent! Message ID: {:?}", result.id);
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use google_gmail1::{
    api::Message,
    hyper::{
        self,
        client::HttpConnector,
        header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
        Body, Method, Request,
    },
    hyper_rustls::{self, HttpsConnector},
    oauth2,
};
use rand::Rng;
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Re-export key types so users don't need to depend on google-gmail1 directly.
pub use google_gmail1::api::Message as GmailMessage;

/// Type alias for the HTTP client used by this library.
pub type GmailHttpClient = hyper::Client<HttpsConnector<HttpConnector>>;

/// Type alias for the OAuth2 authenticator used by this library.
pub type GmailAuthenticator = oauth2::authenticator::Authenticator<HttpsConnector<HttpConnector>>;

/// Get the XDG config directory for gmail-sender.
///
/// Returns `$XDG_CONFIG_HOME/gmail-sender` or `~/.config/gmail-sender` if not set.
pub fn get_config_dir() -> PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("Failed to find home directory")
                .join(".config")
        });
    config_home.join("gmail-sender")
}

/// Get the XDG data directory for gmail-sender.
///
/// Returns `$XDG_DATA_HOME/gmail-sender` or `~/.local/share/gmail-sender` if not set.
pub fn get_data_dir() -> PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("Failed to find home directory")
                .join(".local/share")
        });
    data_home.join("gmail-sender")
}

/// Get a config file path within the XDG config directory.
pub fn get_config_path(filename: &str) -> PathBuf {
    get_config_dir().join(filename)
}

/// Get a data file path within the XDG data directory.
pub fn get_data_path(filename: &str) -> PathBuf {
    get_data_dir().join(filename)
}

/// Validate email header fields to prevent header injection attacks.
///
/// Rejects any input containing CR or LF characters which could be used
/// to inject additional headers.
///
/// # Errors
///
/// Returns an error if the value contains `\r` or `\n` characters.
pub fn validate_header_field(value: &str, field_name: &str) -> Result<()> {
    if value.contains('\r') || value.contains('\n') {
        anyhow::bail!(
            "Invalid {}: contains newline characters (potential header injection)",
            field_name
        );
    }
    Ok(())
}

/// Set secure permissions on a directory (700 on Unix).
#[cfg(unix)]
pub fn set_secure_dir_permissions(path: &PathBuf) -> Result<()> {
    let perms = fs::Permissions::from_mode(0o700);
    fs::set_permissions(path, perms).context(format!("Failed to set permissions on {:?}", path))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_secure_dir_permissions(_path: &PathBuf) -> Result<()> {
    Ok(())
}

/// Set secure permissions on a file (600 on Unix).
#[cfg(unix)]
pub fn set_secure_file_permissions(path: &PathBuf) -> Result<()> {
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perms).context(format!("Failed to set permissions on {:?}", path))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_secure_file_permissions(_path: &PathBuf) -> Result<()> {
    Ok(())
}

/// Check if a path has secure permissions.
///
/// Returns `(is_secure, current_mode)` where `is_secure` is true if
/// the current mode matches `expected_mode`.
#[cfg(unix)]
pub fn check_permissions(path: &PathBuf, expected_mode: u32) -> Result<(bool, u32)> {
    let metadata = fs::metadata(path).context(format!("Failed to read metadata for {:?}", path))?;
    let mode = metadata.permissions().mode() & 0o777;
    Ok((mode == expected_mode, mode))
}

#[cfg(not(unix))]
pub fn check_permissions(_path: &PathBuf, _expected_mode: u32) -> Result<(bool, u32)> {
    Ok((true, 0))
}

/// Warn about insecure permissions on sensitive files/directories.
///
/// Prints a warning to stderr if the file/directory permissions don't match
/// the expected mode.
pub fn warn_insecure_permissions(path: &PathBuf, expected_mode: u32, description: &str) {
    if let Ok((is_secure, current_mode)) = check_permissions(path, expected_mode) {
        if !is_secure {
            eprintln!(
                "⚠ Warning: {} has insecure permissions ({:o}, should be {:o})",
                description, current_mode, expected_mode
            );
            eprintln!("  Fix with: chmod {:o} {:?}", expected_mode, path);
        }
    }
}

/// Create authenticated HTTP client and authenticator for Gmail API.
///
/// This function reads OAuth2 credentials from `client_secret_path` and
/// persists tokens to `token_cache_path`. On first run, it will open
/// a browser for OAuth2 consent.
///
/// # Errors
///
/// Returns an error if the client secret file cannot be read or if
/// authentication fails.
pub async fn create_gmail_client(
    client_secret_path: &PathBuf,
    token_cache_path: &PathBuf,
) -> Result<(GmailHttpClient, GmailAuthenticator)> {
    let secret = oauth2::read_application_secret(client_secret_path)
        .await
        .context(
            "Failed to read client secret file. Did you download it from Google Cloud Console?",
        )?;

    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_cache_path)
    .build()
    .await
    .context("Failed to create authenticator")?;

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

/// Send email via direct HTTP POST to Gmail API.
///
/// Uses the `gmail.send` scope to send the message. The message should
/// be built using [`build_email`].
///
/// # Errors
///
/// Returns an error if the access token cannot be obtained or if the
/// Gmail API request fails.
pub async fn send_email(
    client: &GmailHttpClient,
    auth: &GmailAuthenticator,
    message: Message,
) -> Result<Message> {
    let token = auth
        .token(&["https://www.googleapis.com/auth/gmail.send"])
        .await
        .context("Failed to get access token")?;

    let json_body =
        serde_json::to_string(&message).context("Failed to serialize message to JSON")?;

    let token_str = token
        .token()
        .context("No access token available in response")?;

    let req = Request::builder()
        .method(Method::POST)
        .uri("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
        .header(AUTHORIZATION, format!("Bearer {}", token_str))
        .header(CONTENT_TYPE, "application/json")
        .header(USER_AGENT, "gmail-sender/0.1.0")
        .body(Body::from(json_body))
        .context("Failed to build HTTP request")?;

    let resp = client
        .request(req)
        .await
        .context("Failed to send HTTP request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
        let body_str = String::from_utf8_lossy(&body_bytes);
        anyhow::bail!(
            "Gmail API request failed with status {}: {}",
            status,
            body_str
        );
    }

    let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
    let result: Message =
        serde_json::from_slice(&body_bytes).context("Failed to parse response JSON")?;

    Ok(result)
}

/// Generate a random MIME boundary string.
pub fn generate_boundary() -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 16] = rng.gen();
    format!(
        "boundary_{}",
        random_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    )
}

/// Build an RFC822 email message with optional attachments.
///
/// The returned [`Message`] can be sent using [`send_email`].
///
/// # Important
///
/// The `Message.raw` field is automatically base64url-encoded during
/// JSON serialization by the `google-gmail1` crate. Do not manually
/// encode the raw bytes.
///
/// # Arguments
///
/// * `to` - Recipient email address
/// * `subject` - Email subject line
/// * `body` - Plain text email body
/// * `attachments` - Paths to files to attach
/// * `verbose` - If true, prints debug output to stderr
///
/// # Errors
///
/// Returns an error if an attachment file cannot be read or has an
/// invalid filename.
pub fn build_email(
    to: &str,
    subject: &str,
    body: &str,
    attachments: &[PathBuf],
    verbose: bool,
) -> Result<Message> {
    let boundary = generate_boundary();

    let mut headers = vec![
        "From: me".to_string(),
        format!("To: {}", to),
        format!("Subject: {}", subject),
        "MIME-Version: 1.0".to_string(),
    ];

    let mut email_body = String::new();

    if attachments.is_empty() {
        headers.push("Content-Type: text/plain; charset=UTF-8".to_string());
        email_body = body.to_string();
    } else {
        headers.push(format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"",
            boundary
        ));

        email_body.push_str(&format!("--{}\r\n", boundary));
        email_body.push_str("Content-Type: text/plain; charset=UTF-8\r\n\r\n");
        email_body.push_str(body);
        email_body.push_str("\r\n\r\n");

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

            let encoded_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &file_data);

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

    let raw_email = format!("{}\r\n\r\n{}", headers.join("\r\n"), email_body);

    if verbose {
        eprintln!("DEBUG - Raw email:");
        eprintln!("{}", raw_email.replace("\r\n", "\\r\\n\n"));
        eprintln!("---");
    }

    Ok(Message {
        raw: Some(raw_email.into_bytes()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_header_field_accepts_normal_input() {
        assert!(validate_header_field("test@example.com", "to").is_ok());
        assert!(validate_header_field("Hello World", "subject").is_ok());
    }

    #[test]
    fn test_validate_header_field_rejects_cr() {
        let result = validate_header_field("test\r@example.com", "to");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_header_field_rejects_lf() {
        let result = validate_header_field("test\n@example.com", "to");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_header_field_rejects_crlf() {
        let result = validate_header_field("test\r\n@example.com", "to");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_boundary_is_unique() {
        let b1 = generate_boundary();
        let b2 = generate_boundary();
        assert_ne!(b1, b2);
    }

    #[test]
    fn test_generate_boundary_has_prefix() {
        let boundary = generate_boundary();
        assert!(boundary.starts_with("boundary_"));
    }

    #[test]
    fn test_build_email_simple() {
        let msg = build_email("test@example.com", "Subject", "Body", &[], false).unwrap();
        let raw = String::from_utf8(msg.raw.unwrap()).unwrap();

        assert!(raw.contains("To: test@example.com"));
        assert!(raw.contains("Subject: Subject"));
        assert!(raw.contains("Body"));
        assert!(raw.contains("Content-Type: text/plain"));
    }

    #[test]
    fn test_build_email_uses_crlf() {
        let msg = build_email("test@example.com", "Subject", "Body", &[], false).unwrap();
        let raw = String::from_utf8(msg.raw.unwrap()).unwrap();

        assert!(raw.contains("\r\n"));
        assert!(!raw.contains("\r\n\r\n\r\n"));
    }
}
