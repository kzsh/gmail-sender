# Gmail Sender CLI

A simple command-line tool to send emails via Gmail API using Rust and google-apis-rs.


## Features

- OAuth2 authentication (installed app flow)
- Token caching for subsequent runs
- Command-line arguments with interactive fallback
- File attachments support
- Plain text email body
- XDG Base Directory specification support
- Shell completions generation
- Easy installation with `config install`

## Prerequisites

1. **Rust toolchain** - Install from [rustup.rs](https://rustup.rs/)
2. **Google Cloud Project with Gmail API enabled**

## Setup

### 1. Initialize Configuration Directory

```bash
# Build the project first
cargo build --release

# Initialize the XDG config directory
./target/release/gmail-sender config init
```

This will create:
- `~/.config/gmail-sender/` for configuration files
- `~/.local/share/gmail-sender/` for application data

(or `$XDG_CONFIG_HOME/gmail-sender/` and `$XDG_DATA_HOME/gmail-sender/` if those environment variables are set)

### 2. Create Google Cloud Project

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project (or select an existing one)
3. Enable the Gmail API:
   - Navigate to "APIs & Services" > "Library"
   - Search for "Gmail API"
   - Click "Enable"

### 3. Create OAuth2 Credentials

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
8. Save it as `client_secret.json` in `~/.config/gmail-sender/` (or the path shown by `config init`)

### 4. Optional: Install the Binary

```bash
# Copy to /usr/local/bin (may require sudo)
sudo ./target/release/gmail-sender config install

# OR symlink to ~/.local/bin (no sudo needed)
./target/release/gmail-sender config install --link
```

### 5. Optional: Setup Shell Completions

```bash
# For bash
./target/release/gmail-sender config completions bash | sudo tee /etc/bash_completion.d/gmail-sender

# For zsh (add to a directory in your $fpath)
./target/release/gmail-sender config completions zsh > ~/.zsh/completions/_gmail-sender

# For fish
./target/release/gmail-sender config completions fish > ~/.config/fish/completions/gmail-sender.fish
```

## Usage

### First Run (Authentication)

On first run, the CLI will open your browser for OAuth2 consent:

```bash
gmail-sender
# Or if not installed: ./target/release/gmail-sender
```

You'll be prompted for:
- To: recipient email
- Subject: email subject
- Body: email body

After authentication, a token will be cached in `~/.local/share/gmail-sender/token_cache.json` for future use.

### With Command-Line Arguments

```bash
gmail-sender \
  --to recipient@example.com \
  --subject "Hello from Rust" \
  --body "This is a test email sent via Gmail API"
```

### With Attachments

```bash
gmail-sender \
  --to recipient@example.com \
  --subject "Files attached" \
  --body "Please see attached files" \
  --attachment /path/to/file1.pdf \
  --attachment /path/to/file2.jpg
```

### Mixed Mode (Some Args, Some Prompts)

If you omit arguments, the CLI will prompt for them:

```bash
gmail-sender --to recipient@example.com
# Will prompt for subject and body
```

### Custom File Paths

By default, the tool uses:
- `~/.config/gmail-sender/client_secret.json` for OAuth credentials
- `~/.local/share/gmail-sender/token_cache.json` for cached tokens

You can override these:

```bash
gmail-sender \
  --client-secret /path/to/credentials.json \
  --token-cache /path/to/token.json
```

### Configuration Commands

```bash
# Initialize config directory
gmail-sender config init

# Install the binary
gmail-sender config install [--link]

# Generate shell completions
gmail-sender config completions <SHELL>
```

## Command-Line Options

```
Options:
  -t, --to <TO>                    Recipient email address
  -s, --subject <SUBJECT>          Email subject
  -b, --body <BODY>                Email body (plain text)
  -a, --attachment <ATTACHMENT>    File attachments (can be specified multiple times)
      --client-secret <FILE>       Path to OAuth2 client secret JSON file
                                   [default: $XDG_CONFIG_HOME/gmail-sender/client_secret.json]
      --token-cache <FILE>         Path to store OAuth2 tokens
                                   [default: $XDG_DATA_HOME/gmail-sender/token_cache.json]
  -h, --help                       Print help
  -V, --version                    Print version

Subcommands:
  config                           Configuration commands
    init                           Initialize configuration directory
    install                        Install the executable to system path
    completions                    Generate shell completions
```

## How It Works

1. **Authentication**: Uses OAuth2 installed app flow (opens browser for consent)
2. **Token Storage**: Saves refresh token to disk for reuse
3. **Email Building**: Creates RFC822-formatted email with MIME multipart for attachments
4. **Sending**: Uses Gmail API's `messages.send` endpoint

## Configuration

The tool follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html):

- **Config directory**: `$XDG_CONFIG_HOME/gmail-sender/` (defaults to `~/.config/gmail-sender/`)
  - `client_secret.json` - OAuth2 credentials from Google Cloud Console (user-provided)

- **Data directory**: `$XDG_DATA_HOME/gmail-sender/` (defaults to `~/.local/share/gmail-sender/`)
  - `token_cache.json` - Cached OAuth2 tokens (auto-generated)

You can override these paths with `--client-secret` and `--token-cache` flags.

## Troubleshooting

### "Failed to read client secret file"

Make sure you've downloaded the OAuth2 credentials JSON from Google Cloud Console and saved it in `~/.config/gmail-sender/client_secret.json` (or run `gmail-sender config init` to see the expected path).

### "Access blocked: This app's request is invalid"

Make sure the Gmail API is enabled in your Google Cloud project.

### "This app isn't verified"

Since this is a personal CLI tool, Google will show a warning. Click "Advanced" and "Go to [App Name] (unsafe)" to proceed. For production apps, you'd need to complete Google's verification process.

### Token Issues

If you encounter authentication issues, delete `~/.local/share/gmail-sender/token_cache.json` and re-authenticate.

## Security Notes

- Keep your `client_secret.json` and `token_cache.json` files secure
- Configuration files are stored in `~/.config/gmail-sender/`
- Application data is stored in `~/.local/share/gmail-sender/`
- The OAuth2 tokens grant access to your Gmail account
- Make sure directories have appropriate permissions (600 for files, 700 for directories)

## License

MIT
