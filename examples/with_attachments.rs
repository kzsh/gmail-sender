//! Example of sending an email with attachments using gmail-sender.
//!
//! Run with:
//! ```
//! cargo run --example with_attachments -- /path/to/file1.pdf /path/to/file2.jpg
//! ```

use gmail_sender::{
    build_email, create_gmail_client, get_config_path, get_data_path, send_email,
    validate_header_field,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get attachment paths from command line args
    let attachments: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    if attachments.is_empty() {
        eprintln!("Usage: cargo run --example with_attachments -- <file1> [file2] ...");
        eprintln!("Example: cargo run --example with_attachments -- report.pdf image.png");
        std::process::exit(1);
    }

    // Verify all attachments exist
    for path in &attachments {
        if !path.exists() {
            anyhow::bail!("Attachment not found: {:?}", path);
        }
    }

    let client_secret = get_config_path("client_secret.json");
    let token_cache = get_data_path("token_cache.json");

    let to = "recipient@example.com";
    let subject = "Files attached";
    let body = format!(
        "Please find {} file(s) attached to this email.",
        attachments.len()
    );

    validate_header_field(to, "recipient")?;
    validate_header_field(subject, "subject")?;

    println!("Authenticating...");
    let (client, auth) = create_gmail_client(&client_secret, &token_cache).await?;

    println!("Building email with {} attachment(s)...", attachments.len());
    let message = build_email(to, subject, &body, &attachments, false)?;

    println!("Sending email to {}...", to);
    let result = send_email(&client, &auth, message).await?;

    println!("Email sent successfully!");
    println!("  Message ID: {:?}", result.id);

    Ok(())
}
