//! Simple example of sending an email using gmail-sender as a library.
//!
//! Run with:
//! ```
//! cargo run --example simple_send
//! ```
//!
//! Make sure you have:
//! 1. Run `gmail-sender config init` to set up directories
//! 2. Placed your `client_secret.json` in the config directory

use gmail_sender::{
    build_email, create_gmail_client, get_config_path, get_data_path, send_email,
    validate_header_field,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use default XDG paths for credentials
    let client_secret = get_config_path("client_secret.json");
    let token_cache = get_data_path("token_cache.json");

    // Email details - in a real app, get these from user input or config
    let to = "recipient@example.com";
    let subject = "Hello from gmail-sender";
    let body = "This email was sent using the gmail-sender library.";

    // Validate inputs to prevent header injection
    validate_header_field(to, "recipient")?;
    validate_header_field(subject, "subject")?;

    // Create authenticated client
    println!("Authenticating...");
    let (client, auth) = create_gmail_client(&client_secret, &token_cache).await?;

    // Build the email message
    let message = build_email(to, subject, body, &[], false)?;

    // Send it
    println!("Sending email to {}...", to);
    let result = send_email(&client, &auth, message).await?;

    println!("Email sent successfully!");
    println!("  Message ID: {:?}", result.id);
    println!("  Thread ID: {:?}", result.thread_id);

    Ok(())
}
