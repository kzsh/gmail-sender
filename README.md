# Gmail Sender CLI

A simple command-line tool to send emails via Gmail API using Rust and google-apis-rs.


## Features

- OAuth2 authentication (installed app flow)
- Token caching for subsequent runs
- Command-line arguments with interactive fallback
- File attachments support
- Plain text email body

## Prerequisites

1. **Rust toolchain** - Install from [rustup.rs](https://rustup.rs/)
2. **Google Cloud Project with Gmail API enabled**

## Setup

### 1. Create Google Cloud Project

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project (or select an existing one)
3. Enable the Gmail API:
   - Navigate to "APIs & Services" > "Library"
   - Search for "Gmail API"
   - Click "Enable"

### 2. Create OAuth2 Credentials

1. Go to "APIs & Services" > "Credentials"
2. Click "Create Credentials" > "OAuth client ID"
3. If prompted, configure the OAuth consent screen:
   - Choose "External" user type
   - Fill in required fields (app name, user support email, developer email)
   - Add your email to "Test users"
4. For application type, choose "Desktop app"
5. Give it a name (e.g., "Gmail Sender CLI")
6. Click "Create"
7. Download the JSON file
8. Save it as `client_secret.json` in the project directory

### 3. Build the Project

```bash
cd gmail-sender
cargo build --release
```

The binary will be at `target/release/gmail-sender`

## Usage

### First Run (Authentication)

On first run, the CLI will open your browser for OAuth2 consent:

```bash
./target/release/gmail-sender
```

You'll be prompted for:
- To: recipient email
- Subject: email subject
- Body: email body

After authentication, a token will be cached in `token_cache.json` for future use.

### With Command-Line Arguments

```bash
./target/release/gmail-sender \
  --to recipient@example.com \
  --subject "Hello from Rust" \
  --body "This is a test email sent via Gmail API"
```

### With Attachments

```bash
./target/release/gmail-sender \
  --to recipient@example.com \
  --subject "Files attached" \
  --body "Please see attached files" \
  --attachment /path/to/file1.pdf \
  --attachment /path/to/file2.jpg
```

### Mixed Mode (Some Args, Some Prompts)

If you omit arguments, the CLI will prompt for them:

```bash
./target/release/gmail-sender --to recipient@example.com
# Will prompt for subject and body
```

### Custom Credentials Path

```bash
./target/release/gmail-sender \
  --client-secret /path/to/credentials.json \
  --token-cache /path/to/token.json
```

## Command-Line Options

```
Options:
  -t, --to <TO>                    Recipient email address
  -s, --subject <SUBJECT>          Email subject
  -b, --body <BODY>                Email body (plain text)
  -a, --attachment <ATTACHMENT>    File attachments (can be specified multiple times)
      --client-secret <FILE>       Path to OAuth2 client secret JSON file [default: client_secret.json]
      --token-cache <FILE>         Path to store OAuth2 tokens [default: token_cache.json]
  -h, --help                       Print help
  -V, --version                    Print version
```

## How It Works

1. **Authentication**: Uses OAuth2 installed app flow (opens browser for consent)
2. **Token Storage**: Saves refresh token to disk for reuse
3. **Email Building**: Creates RFC822-formatted email with MIME multipart for attachments
4. **Sending**: Uses Gmail API's `messages.send` endpoint

## Troubleshooting

### "Failed to read client secret file"

Make sure you've downloaded the OAuth2 credentials JSON from Google Cloud Console and saved it as `client_secret.json` in the project directory (or specify the path with `--client-secret`).

### "Access blocked: This app's request is invalid"

Make sure the Gmail API is enabled in your Google Cloud project.

### "This app isn't verified"

Since this is a personal CLI tool, Google will show a warning. Click "Advanced" and "Go to [App Name] (unsafe)" to proceed. For production apps, you'd need to complete Google's verification process.

### Token Issues

If you encounter authentication issues, delete `token_cache.json` and re-authenticate.

## Security Notes

- Keep your `client_secret.json` and `token_cache.json` files secure
- Don't commit these files to version control (add them to `.gitignore`)
- The OAuth2 tokens grant access to your Gmail account

## License

MIT
