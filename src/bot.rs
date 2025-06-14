use anyhow::Result;
use log::{error, info, warn};
use serde_json::Value;
use std::process::Stdio;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::io::{AsyncBufReadExt, BufReader};
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
                let warning = "🚨 **FOOTGUN ALERT: SERIOUS SECURITY RISKS** 🚨\n\n\
                              This bot executes Claude commands DIRECTLY on your host system with NO SANDBOXING.\n\n\
                              **POTENTIAL RISKS:**\n\
                              • **FULL SYSTEM COMPROMISE** - Claude can execute ANY command on your system\n\
                              • **DATA EXFILTRATION** - Access to all files and system resources\n\
                              • **REMOTE CODE EXECUTION** - Potential for malicious code execution\n\
                              • **PRIVILEGE ESCALATION** - Commands run with your user privileges\n\n\
                              **RECOMMENDED PRECAUTIONS:**\n\
                              • Run in a isolated Docker container or VM\n\
                              • Use a dedicated, restricted user account\n\
                              • Monitor all system activity\n\
                              • Never run on production systems\n\n\
                              **USE AT YOUR OWN RISK!**\n\n\
                              Send any message to execute Claude commands.";

                bot.send_message(msg.chat.id, warning)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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

    let mut child = Command::new("claude")
        .arg("--dangerously-skip-permissions")
        .arg("--output-format")
        .arg("json")
        .arg(prompt)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut output = String::new();
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        if let Ok(json_value) = serde_json::from_str::<Value>(&line) {
            if let Some(content) = json_value.get("content") {
                if let Some(text) = content.as_str() {
                    output.push_str(text);
                }
            } else if let Some(error) = json_value.get("error") {
                if let Some(error_text) = error.as_str() {
                    return Err(anyhow::anyhow!("Claude error: {}", error_text));
                }
            }
        }
        line.clear();
    }

    let exit_status = child.wait().await?;
    if !exit_status.success() {
        return Err(anyhow::anyhow!(
            "Claude command failed with exit code: {}",
            exit_status
        ));
    }

    Ok(output)
}
