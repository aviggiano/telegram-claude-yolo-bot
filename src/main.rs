use clap::{Parser, Subcommand};
use std::env;
use std::process;

mod bot;
mod config;
mod daemon;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Telegram bot
    Start {
        /// Telegram bot token (optional if set in .env)
        #[arg(short, long)]
        token: Option<String>,
        /// Authorized chat ID (optional if set in .env)
        #[arg(short, long)]
        chat_id: Option<i64>,
        /// Run as daemon
        #[arg(short, long)]
        daemon: bool,
    },
    /// Install the bot as a system daemon
    Install {
        /// Telegram bot token (optional if set in .env)
        #[arg(short, long)]
        token: Option<String>,
        /// Authorized chat ID (optional if set in .env)
        #[arg(short, long)]
        chat_id: Option<i64>,
    },
    /// Uninstall the system daemon
    Uninstall,
    /// Show daemon status
    Status,
}

#[tokio::main]
async fn main() {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            token,
            chat_id,
            daemon,
        } => {
            let (token, chat_id) = match get_config_values(token, chat_id) {
                Ok(values) => values,
                Err(e) => {
                    eprintln!("Configuration error: {}", e);
                    process::exit(1);
                }
            };

            if daemon {
                if let Err(e) = daemon::start_daemon(token, chat_id).await {
                    eprintln!("Failed to start daemon: {}", e);
                    process::exit(1);
                }
            } else {
                if let Err(e) = bot::start_bot(token, chat_id).await {
                    eprintln!("Failed to start bot: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Install { token, chat_id } => {
            let (token, chat_id) = match get_config_values(token, chat_id) {
                Ok(values) => values,
                Err(e) => {
                    eprintln!("Configuration error: {}", e);
                    process::exit(1);
                }
            };

            if let Err(e) = daemon::install_daemon(token, chat_id) {
                eprintln!("Failed to install daemon: {}", e);
                process::exit(1);
            }
            println!("Daemon installed successfully");
        }
        Commands::Uninstall => {
            if let Err(e) = daemon::uninstall_daemon() {
                eprintln!("Failed to uninstall daemon: {}", e);
                process::exit(1);
            }
            println!("Daemon uninstalled successfully");
        }
        Commands::Status => {
            daemon::show_status();
        }
    }
}

fn get_config_values(
    token_arg: Option<String>,
    chat_id_arg: Option<i64>,
) -> Result<(String, i64), String> {
    let token = token_arg
        .or_else(|| env::var("TELEGRAM_BOT_TOKEN").ok())
        .ok_or_else(|| {
            "Telegram bot token not provided. Set TELEGRAM_BOT_TOKEN environment variable or use --token".to_string()
        })?;

    let chat_id = chat_id_arg
        .or_else(|| {
            env::var("TELEGRAM_CHAT_ID")
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
        })
        .ok_or_else(|| {
            "Telegram chat ID not provided. Set TELEGRAM_CHAT_ID environment variable or use --chat-id".to_string()
        })?;

    Ok((token, chat_id))
}
