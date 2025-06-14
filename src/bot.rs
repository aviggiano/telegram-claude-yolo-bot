use crate::updater::AutoUpdater;
use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use std::fs::OpenOptions;
use std::io::Write;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::process::Command;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum BotCommand {
    #[command(description = "Display this help message")]
    Help,
    #[command(description = "Start the bot")]
    Start,
}

pub async fn start_bot(token: String, authorized_chat_id: i64) -> Result<()> {
    info!("Starting Telegram Claude YOLO Bot...");

    let bot = Bot::new(token);

    // Start auto-updater in background (check every 5 minutes)
    let updater = AutoUpdater::new(5);
    tokio::spawn(async move {
        updater.start_monitoring().await;
    });

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let chat_id = authorized_chat_id;
        async move { handle_message(bot, msg, chat_id).await }
    })
    .await;

    Ok(())
}

async fn handle_message(bot: Bot, msg: Message, authorized_chat_id: i64) -> ResponseResult<()> {
    if msg.chat.id.0 != authorized_chat_id {
        warn!(
            "Unauthorized access attempt from chat ID: {}",
            msg.chat.id.0
        );
        return Ok(());
    }

    let text = match msg.text() {
        Some(text) => text,
        None => return Ok(()),
    };

    if let Ok(command) = BotCommand::parse(text, "telegram-claude-yolo-bot") {
        match command {
            BotCommand::Help => {
                let help_text = BotCommand::descriptions().to_string();
                log_to_screenlog("BOT", &help_text).ok();
                bot.send_message(msg.chat.id, help_text).await?;
            }
            BotCommand::Start => {
                let start_text =
                    "Claude YOLO Bot is ready! Send any message to execute Claude commands.";
                log_to_screenlog("BOT", start_text).ok();
                bot.send_message(msg.chat.id, start_text).await?;
            }
        }
    } else {
        // Log user input
        log_to_screenlog("USER", text).ok();

        // Execute Claude command
        match execute_claude_command(text).await {
            Ok(output) => {
                if output.is_empty() {
                    let no_output_msg = "No output received from Claude.";
                    log_to_screenlog("BOT", no_output_msg).ok();
                    bot.send_message(msg.chat.id, no_output_msg).await?;
                } else {
                    // Log Claude's full output
                    log_to_screenlog("CLAUDE", &output).ok();

                    // Split long messages to respect Telegram's 4096 character limit
                    for chunk in output.chars().collect::<Vec<char>>().chunks(4000) {
                        let chunk_str: String = chunk.iter().collect();
                        bot.send_message(msg.chat.id, chunk_str).await?;
                    }
                }
            }
            Err(e) => {
                error!("Claude command failed: {}", e);
                let error_msg = format!("Error: {}", e);
                log_to_screenlog("BOT", &error_msg).ok();
                bot.send_message(msg.chat.id, error_msg).await?;
            }
        }
    }

    Ok(())
}

async fn execute_claude_command(prompt: &str) -> Result<String> {
    info!("Executing Claude command: {}", prompt);

    let output = Command::new("claude")
        .arg("--dangerously-skip-permissions")
        .arg(prompt)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Claude command failed with exit code: {}\nError: {}",
            output.status,
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

fn log_to_screenlog(message_type: &str, content: &str) -> Result<()> {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let log_entry = format!("[{}] {}: {}\n", timestamp, message_type, content);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("screenlog.0")?;

    file.write_all(log_entry.as_bytes())?;
    file.flush()?;

    Ok(())
}
