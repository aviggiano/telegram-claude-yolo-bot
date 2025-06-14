use clap::{Parser, Subcommand};
use std::process;

mod bot;
mod daemon;
mod config;

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
        /// Telegram bot token
        #[arg(short, long, env = "TELEGRAM_BOT_TOKEN")]
        token: String,
        /// Authorized chat ID
        #[arg(short, long, env = "TELEGRAM_CHAT_ID")]
        chat_id: i64,
        /// Run as daemon
        #[arg(short, long)]
        daemon: bool,
    },
    /// Install the bot as a system daemon
    Install {
        /// Telegram bot token
        #[arg(short, long, env = "TELEGRAM_BOT_TOKEN")]
        token: String,
        /// Authorized chat ID
        #[arg(short, long, env = "TELEGRAM_CHAT_ID")]
        chat_id: i64,
    },
    /// Uninstall the system daemon
    Uninstall,
    /// Show daemon status
    Status,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start { token, chat_id, daemon } => {
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