use clap::{Parser, Subcommand};
use std::env;
use std::process;

mod bot;
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
    Start,
}

#[tokio::main]
async fn main() {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            let (token, chat_id) = match get_config_values() {
                Ok(values) => values,
                Err(e) => {
                    eprintln!("Configuration error: {}", e);
                    process::exit(1);
                }
            };

            if let Err(e) = bot::start_bot(token, chat_id).await {
                eprintln!("Failed to start bot: {}", e);
                process::exit(1);
            }
        }
    }
}

fn get_config_values() -> Result<(String, i64), String> {
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| "Telegram bot token not provided. Set TELEGRAM_BOT_TOKEN environment variable".to_string())?;

    let chat_id = env::var("TELEGRAM_CHAT_ID")
        .map_err(|_| "Telegram chat ID not provided. Set TELEGRAM_CHAT_ID environment variable".to_string())?
        .parse::<i64>()
        .map_err(|_| "Invalid TELEGRAM_CHAT_ID format. Must be a valid integer".to_string())?;

    Ok((token, chat_id))
}
