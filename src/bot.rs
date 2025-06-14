use anyhow::Result;
use log::{error, info, warn};
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
                bot.send_message(msg.chat.id, BotCommand::descriptions().to_string())
                    .await?;
            }
            BotCommand::Start => {
                bot.send_message(
                    msg.chat.id,
                    "Claude YOLO Bot is ready! Send any message to execute Claude commands.",
                )
                .await?;
            }
        }
    } else {
        // Execute Claude command
        match execute_claude_command(text).await {
            Ok(output) => {
                if output.is_empty() {
                    bot.send_message(msg.chat.id, "No output received from Claude.")
                        .await?;
                } else {
                    // Split long messages to respect Telegram's 4096 character limit
                    for chunk in output.chars().collect::<Vec<char>>().chunks(4000) {
                        let chunk_str: String = chunk.iter().collect();
                        bot.send_message(msg.chat.id, chunk_str).await?;
                    }
                }
            }
            Err(e) => {
                error!("Claude command failed: {}", e);
                bot.send_message(msg.chat.id, format!("Error: {}", e))
                    .await?;
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
