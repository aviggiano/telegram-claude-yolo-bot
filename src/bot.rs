use crate::updater::AutoUpdater;
use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use std::fs::OpenOptions;
use std::io::Write;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

fn escape_markdown_v2(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|'
            | '{' | '}' | '.' | '!' => {
                format!("\\{}", c)
            }
            c => c.to_string(),
        })
        .collect()
}

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
                bot.send_message(msg.chat.id, escape_markdown_v2(&help_text))
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
            BotCommand::Start => {
                let start_text =
                    "Claude YOLO Bot is ready! Send any message to execute Claude commands.";
                log_to_screenlog("BOT", start_text).ok();
                bot.send_message(msg.chat.id, escape_markdown_v2(start_text))
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await?;
            }
        }
    } else {
        // Log user input
        log_to_screenlog("USER", text).ok();

        // Execute Claude command with real-time streaming
        if let Err(e) = execute_claude_command_streaming(text, bot.clone(), msg.chat.id).await {
            error!("Claude command failed: {}", e);
            let error_msg = format!("Error: {}", e);
            log_to_screenlog("BOT", &error_msg).ok();
            bot.send_message(msg.chat.id, escape_markdown_v2(&error_msg))
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
        }
    }

    Ok(())
}

async fn execute_claude_command_streaming(prompt: &str, bot: Bot, chat_id: ChatId) -> Result<()> {
    info!("Executing Claude command with streaming: {}", prompt);

    let mut child = Command::new("claude")
        .arg("--dangerously-skip-permissions")
        .arg(prompt)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;

    let mut reader = BufReader::new(stdout).lines();
    let mut accumulated_output = String::new();
    let mut buffer = String::new();
    let mut last_send_time = std::time::Instant::now();

    // Send initial message to show bot is processing
    let mut current_message = bot
        .send_message(chat_id, escape_markdown_v2("🤖 Processing..."))
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await?;

    while let Some(line) = reader.next_line().await? {
        buffer.push_str(&line);
        buffer.push('\n');
        accumulated_output.push_str(&line);
        accumulated_output.push('\n');

        // Send updates every 1 second or when buffer gets large
        let should_send = last_send_time.elapsed().as_secs() >= 1 || buffer.len() > 2000;

        if should_send && !buffer.trim().is_empty() {
            // Edit the message with accumulated output
            let display_text = if accumulated_output.len() > 4000 {
                escape_markdown_v2(&format!(
                    "...{}",
                    &accumulated_output[accumulated_output.len() - 3900..]
                ))
            } else {
                escape_markdown_v2(&accumulated_output)
            };

            if let Err(_e) = bot
                .edit_message_text(chat_id, current_message.id, display_text)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await
            {
                // If edit fails (message too old or identical), send new message
                match bot
                    .send_message(chat_id, escape_markdown_v2(&buffer))
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await
                {
                    Ok(new_msg) => current_message = new_msg,
                    Err(send_err) => warn!("Failed to send message: {}", send_err),
                }
            }

            buffer.clear();
            last_send_time = std::time::Instant::now();
        }
    }

    // Send any remaining buffer content
    if !buffer.trim().is_empty() {
        let display_text = if accumulated_output.len() > 4000 {
            escape_markdown_v2(&format!(
                "...{}",
                &accumulated_output[accumulated_output.len() - 3900..]
            ))
        } else {
            escape_markdown_v2(&accumulated_output)
        };

        if let Err(_e) = bot
            .edit_message_text(chat_id, current_message.id, display_text)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await
        {
            bot.send_message(chat_id, escape_markdown_v2(&buffer))
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
        }
    }

    // Wait for the process to complete and check exit status
    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Claude command failed with exit code: {}\nError: {}",
            output.status,
            stderr
        ));
    }

    // Log the complete output
    log_to_screenlog("CLAUDE", &accumulated_output).ok();

    // Send final completion message if no output was received
    if accumulated_output.trim().is_empty() {
        let no_output_msg = "✅ Claude command completed (no output)";
        bot.edit_message_text(chat_id, current_message.id, escape_markdown_v2(no_output_msg))
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
    }

    Ok(())
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
