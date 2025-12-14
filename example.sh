#!/bin/bash
# Example script demonstrating gmail-sender usage

# Basic usage with all arguments
./target/release/gmail-sender \
  --to recipient@example.com \
  --subject "Hello from Rust CLI" \
  --body "This is a test email sent via the Gmail API using Byron's google-apis-rs library."

# With attachments
./target/release/gmail-sender \
  --to recipient@example.com \
  --subject "Files attached" \
  --body "Please find the attached files." \
  --attachment /path/to/document.pdf \
  --attachment /path/to/image.jpg

# Interactive mode (prompts for missing fields)
./target/release/gmail-sender --to recipient@example.com
# Will prompt for subject and body

# Fully interactive
./target/release/gmail-sender
# Will prompt for to, subject, and body
