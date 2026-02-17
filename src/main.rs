use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use dialoguer::Input;
use gmail_sender::{
    build_email, check_permissions, create_gmail_client, get_config_dir, get_config_path,
    get_data_dir, get_data_path, send_email, set_secure_dir_permissions,
    set_secure_file_permissions, validate_header_field, warn_insecure_permissions,
};
use std::fs;
use std::path::PathBuf;

/// A simple CLI tool to send emails via Gmail API
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Recipient email address
    #[arg(short, long, global = true)]
    to: Option<String>,

    /// Email subject
    #[arg(short, long, global = true)]
    subject: Option<String>,

    /// Email body (plain text)
    #[arg(short, long, global = true)]
    body: Option<String>,

    /// File attachments (can be specified multiple times)
    #[arg(short, long, global = true)]
    attachment: Vec<PathBuf>,

    /// Path to OAuth2 client secret JSON file (defaults to $XDG_CONFIG_HOME/gmail-sender/client_secret.json)
    #[arg(long, global = true)]
    client_secret: Option<PathBuf>,

    /// Path to store OAuth2 tokens (defaults to $XDG_DATA_HOME/gmail-sender/token_cache.json)
    #[arg(long, global = true)]
    token_cache: Option<PathBuf>,

    /// Enable verbose output (shows debug information including raw email content)
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Configuration commands
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Initialize configuration directory
    Init,
    /// Check configuration setup
    Check,
    /// Install the executable to system path
    Install {
        /// Create symlink to ~/.local/bin/ instead of copying to /usr/local/bin/
        #[arg(long)]
        link: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(command) = args.command {
        match command {
            Commands::Config { config_command } => match config_command {
                ConfigCommands::Init => {
                    return handle_init();
                }
                ConfigCommands::Check => {
                    return handle_check();
                }
                ConfigCommands::Install { link } => {
                    return handle_install(link);
                }
                ConfigCommands::Completions { shell } => {
                    return handle_completions(shell);
                }
            },
        }
    }

    let client_secret = args
        .client_secret
        .unwrap_or_else(|| get_config_path("client_secret.json"));
    let token_cache = args
        .token_cache
        .unwrap_or_else(|| get_data_path("token_cache.json"));

    let to = get_or_prompt(args.to, "To")?;
    let subject = get_or_prompt(args.subject, "Subject")?;
    let body = get_or_prompt(args.body, "Body")?;

    validate_header_field(&to, "recipient address")?;
    validate_header_field(&subject, "subject")?;

    let config_dir = get_config_dir();
    let data_dir = get_data_dir();
    if config_dir.exists() {
        warn_insecure_permissions(&config_dir, 0o700, "Config directory");
    }
    if data_dir.exists() {
        warn_insecure_permissions(&data_dir, 0o700, "Data directory");
    }
    if token_cache.exists() {
        warn_insecure_permissions(&token_cache, 0o600, "Token cache");
    }

    println!("Authenticating with Gmail...");
    let (client, auth) = create_gmail_client(&client_secret, &token_cache).await?;

    if token_cache.exists() {
        if let Err(e) = set_secure_file_permissions(&token_cache) {
            eprintln!(
                "⚠ Warning: Could not set secure permissions on token cache: {}",
                e
            );
        }
    }

    println!("Building email...");
    let email_message = build_email(&to, &subject, &body, &args.attachment, args.verbose)?;

    println!("Sending email...");
    let result = send_email(&client, &auth, email_message).await?;

    println!("✓ Email sent successfully!");
    println!("  Message ID: {}", result.id.unwrap_or_default());
    println!("  Thread ID: {}", result.thread_id.unwrap_or_default());

    Ok(())
}

fn handle_init() -> Result<()> {
    let config_dir = get_config_dir();
    let data_dir = get_data_dir();

    if config_dir.exists() {
        println!("Configuration directory already exists at {:?}", config_dir);
        if let Ok((is_secure, _)) = check_permissions(&config_dir, 0o700) {
            if !is_secure {
                set_secure_dir_permissions(&config_dir)?;
                println!("  ✓ Fixed permissions to 700");
            }
        }
    } else {
        fs::create_dir_all(&config_dir)
            .context(format!("Failed to create directory: {:?}", config_dir))?;
        set_secure_dir_permissions(&config_dir)?;
        println!(
            "✓ Created configuration directory at {:?} (mode 700)",
            config_dir
        );
    }

    if data_dir.exists() {
        println!("Data directory already exists at {:?}", data_dir);
        if let Ok((is_secure, _)) = check_permissions(&data_dir, 0o700) {
            if !is_secure {
                set_secure_dir_permissions(&data_dir)?;
                println!("  ✓ Fixed permissions to 700");
            }
        }
    } else {
        fs::create_dir_all(&data_dir)
            .context(format!("Failed to create directory: {:?}", data_dir))?;
        set_secure_dir_permissions(&data_dir)?;
        println!("✓ Created data directory at {:?} (mode 700)", data_dir);
    }

    println!("\nExpected file locations:");
    println!(
        "  Client Secret: {:?}",
        get_config_path("client_secret.json")
    );
    println!("  Token Cache:   {:?}", get_data_path("token_cache.json"));

    let client_secret_path = get_config_path("client_secret.json");
    if !client_secret_path.exists() {
        println!("\n⚠ Warning: client_secret.json not found");
        println!("  Download it from Google Cloud Console and place it at:");
        println!("  {:?}", client_secret_path);
    } else {
        println!("\n✓ client_secret.json found");
    }

    Ok(())
}

fn handle_check() -> Result<()> {
    let config_dir = get_config_dir();
    let data_dir = get_data_dir();
    let client_secret_path = get_config_path("client_secret.json");
    let token_cache_path = get_data_path("token_cache.json");

    let mut all_ok = true;

    println!("Checking gmail-sender configuration...\n");

    print!("Config directory ({:?}): ", config_dir);
    if config_dir.exists() {
        match check_permissions(&config_dir, 0o700) {
            Ok((true, _)) => println!("✓ exists (mode 700)"),
            Ok((false, mode)) => {
                println!("⚠ exists but insecure (mode {:o}, should be 700)", mode);
                println!("  Run 'gmail-sender config init' to fix permissions");
            }
            Err(_) => println!("✓ exists"),
        }
    } else {
        println!("✗ missing");
        println!("  Run 'gmail-sender config init' to create it");
        all_ok = false;
    }

    print!("Data directory ({:?}): ", data_dir);
    if data_dir.exists() {
        match check_permissions(&data_dir, 0o700) {
            Ok((true, _)) => println!("✓ exists (mode 700)"),
            Ok((false, mode)) => {
                println!("⚠ exists but insecure (mode {:o}, should be 700)", mode);
                println!("  Run 'gmail-sender config init' to fix permissions");
            }
            Err(_) => println!("✓ exists"),
        }
    } else {
        println!("✗ missing");
        println!("  Run 'gmail-sender config init' to create it");
        all_ok = false;
    }

    print!("\nClient secret ({:?}): ", client_secret_path);
    if !client_secret_path.exists() {
        println!("✗ not found");
        println!("  Download OAuth2 credentials from Google Cloud Console");
        println!("  and save as {:?}", client_secret_path);
        all_ok = false;
    } else {
        match fs::read_to_string(&client_secret_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json) => {
                    let has_installed = json.get("installed").is_some();
                    let has_web = json.get("web").is_some();

                    if has_installed || has_web {
                        let client_type = if has_installed {
                            json.get("installed")
                        } else {
                            json.get("web")
                        };

                        if let Some(client) = client_type {
                            let has_client_id = client.get("client_id").is_some();
                            let has_client_secret = client.get("client_secret").is_some();

                            if has_client_id && has_client_secret {
                                println!("✓ valid");
                            } else {
                                println!("✗ invalid (missing client_id or client_secret)");
                                all_ok = false;
                            }
                        } else {
                            println!("✗ invalid structure");
                            all_ok = false;
                        }
                    } else {
                        println!("✗ invalid (missing 'installed' or 'web' section)");
                        println!("  Make sure you downloaded a Desktop app or Web app credential");
                        all_ok = false;
                    }
                }
                Err(e) => {
                    println!("✗ invalid JSON: {}", e);
                    all_ok = false;
                }
            },
            Err(e) => {
                println!("✗ cannot read: {}", e);
                all_ok = false;
            }
        }
    }

    print!("\nToken cache ({:?}): ", token_cache_path);
    if token_cache_path.exists() {
        match check_permissions(&token_cache_path, 0o600) {
            Ok((true, _)) => println!("✓ exists (mode 600, you're authenticated)"),
            Ok((false, mode)) => {
                println!("⚠ exists but insecure (mode {:o}, should be 600)", mode);
                println!("  Fix with: chmod 600 {:?}", token_cache_path);
            }
            Err(_) => println!("✓ exists (you're authenticated)"),
        }
    } else {
        println!("⚬ not found (will be created on first run)");
    }

    println!(
        "\n{}",
        if all_ok {
            "✓ Configuration is valid! You're ready to send emails."
        } else {
            "✗ Configuration has issues. Please fix the problems above."
        }
    );

    if all_ok {
        Ok(())
    } else {
        anyhow::bail!("Configuration check failed")
    }
}

fn handle_install(use_link: bool) -> Result<()> {
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    let exe_name = current_exe
        .file_name()
        .context("Failed to get executable name")?;

    if use_link {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let target_dir = PathBuf::from(home).join(".local/bin");

        fs::create_dir_all(&target_dir)
            .context(format!("Failed to create directory: {:?}", target_dir))?;

        let target_path = target_dir.join(exe_name);

        if target_path.exists() {
            fs::remove_file(&target_path)
                .context(format!("Failed to remove existing file: {:?}", target_path))?;
        }

        std::os::unix::fs::symlink(&current_exe, &target_path)
            .context(format!("Failed to create symlink to {:?}", target_path))?;

        println!("✓ Symlinked to {:?}", target_path);
        println!("  Make sure {:?} is in your PATH", target_dir);
    } else {
        let target_dir = PathBuf::from("/usr/local/bin");
        let target_path = target_dir.join(exe_name);

        fs::copy(&current_exe, &target_path).context(format!(
            "Failed to copy to {:?}. You may need to run with sudo.",
            target_path
        ))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&target_path, perms)
                .context(format!("Failed to set permissions on {:?}", target_path))?;
        }

        println!("✓ Installed to {:?}", target_path);
    }

    Ok(())
}

fn handle_completions(shell: Shell) -> Result<()> {
    let mut cmd = Args::command();
    let bin_name = cmd.get_name().to_string();

    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());

    Ok(())
}

fn get_or_prompt(value: Option<String>, field_name: &str) -> Result<String> {
    match value {
        Some(v) => Ok(v),
        None => Input::new()
            .with_prompt(field_name)
            .interact_text()
            .context(format!("Failed to read {}", field_name)),
    }
}
