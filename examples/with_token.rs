//! Example of sending an email using a pre-obtained OAuth2 token.
//!
//! This is useful when your application manages its own authentication,
//! such as using service accounts, a centralized auth service, or tokens
//! obtained via a different OAuth2 flow.
//!
//! Run with:
//! ```
//! GMAIL_ACCESS_TOKEN="your_token_here" cargo run --example with_token
//! ```
//!
//! The token must have the `gmail.send` scope.

use gmail_sender::{build_email, create_http_client, send_email_with_token, validate_header_field};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get token from environment (in real usage, this might come from
    // a secrets manager, service account, or your auth system)
    let token = std::env::var("GMAIL_ACCESS_TOKEN").map_err(|_| {
        anyhow::anyhow!(
            "GMAIL_ACCESS_TOKEN environment variable not set.\n\
             Set it to a valid OAuth2 token with gmail.send scope."
        )
    })?;

    let to = "recipient@example.com";
    let subject = "Hello from gmail-sender (BYOT)";
    let body = "This email was sent using a pre-obtained OAuth2 token.";

    validate_header_field(to, "recipient")?;
    validate_header_field(subject, "subject")?;

    // Create HTTP client (no authenticator needed)
    let client = create_http_client()?;

    // Build and send
    let message = build_email(to, subject, body, &[], false)?;

    println!("Sending email to {}...", to);
    let result = send_email_with_token(&client, &token, message).await?;

    println!("Email sent successfully!");
    println!("  Message ID: {:?}", result.id);
    println!("  Thread ID: {:?}", result.thread_id);

    Ok(())
}
