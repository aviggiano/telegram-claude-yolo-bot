# Telegram Claude YOLO Bot

A Rust-based Telegram bot that bridges Claude CLI interactions, allowing you to interact with Claude through Telegram messages.

## Installation

Install directly from GitHub using cargo:

```bash
cargo install --git https://github.com/aviggiano/telegram-claude-yolo-bot
```

### From source

```bash
git clone https://github.com/aviggiano/telegram-claude-yolo-bot
cd telegram-claude-yolo-bot
cargo install --path .
```

## Prerequisites

1. **Claude CLI**: Make sure you have the Claude CLI installed and configured
2. **Telegram Bot Token**: Create a bot through [@BotFather](https://t.me/BotFather)
3. **Chat ID**: Get your Telegram chat ID (you can use [@userinfobot](https://t.me/userinfobot))

## Usage

### Using .env File (Recommended)

Create a `.env` file in your project directory:

```env
TELEGRAM_BOT_TOKEN=your_bot_token_here
TELEGRAM_CHAT_ID=your_chat_id_here
```

Then start the bot:

```bash
telegram-claude-yolo-bot start
```

### Using Environment Variables

Alternatively, set environment variables:

```bash
export TELEGRAM_BOT_TOKEN=your_bot_token_here
export TELEGRAM_CHAT_ID=your_chat_id_here
telegram-claude-yolo-bot start
```

### Running with Screen (Recommended)

To keep the bot running in the background, use screen:

```bash
screen -d -L -m telegram-claude-yolo-bot start
```

This will:
- `-d`: Start screen detached
- `-L`: Log output to a file
- `-m`: Force creation of a new session

## Commands

| Command | Description |
|---------|-------------|
| `start` | Start the Telegram bot |


## Telegram Bot Commands

- `/start` - Display security warning and help
- `/help` - Show available commands
- Send any other message to execute as a Claude command

## Configuration

The bot can be configured using:

1. `.env` file in the current directory
2. Environment variables

### Configuration Priority

Values are loaded in this order (later values override earlier ones):
1. Environment variables
2. `.env` file

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

### Logging

Set `RUST_LOG` environment variable for logging:

```bash
RUST_LOG=info telegram-claude-yolo-bot start
```

## Docker Usage (Recommended)

Create a Dockerfile:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
# Install Claude CLI here
COPY --from=builder /app/target/release/telegram-claude-yolo-bot /usr/local/bin/
CMD ["telegram-claude-yolo-bot", "start"]
```

Run with Docker:

```bash
docker build -t telegram-claude-bot .
docker run -e TELEGRAM_BOT_TOKEN=your_token -e TELEGRAM_CHAT_ID=your_chat_id telegram-claude-bot
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

This project is based on the original Python implementation by [@devtooligan](https://x.com/devtooligan). You can find the original gist [here](https://gist.github.com/devtooligan/998d1405dfa11085e4d022bb98ded85a).

## 🚨 Security Warning

**This bot executes Claude commands DIRECTLY on your host system with NO SANDBOXING.**

### Potential Risks:
- **Full system compromise** - Claude can execute ANY command on your system
- **Data exfiltration** - Access to all files and system resources  
- **Remote code execution** - Potential for malicious code execution
- **Privilege escalation** - Commands run with your user privileges

### Recommended Precautions:
- Run in an isolated Docker container or VM
- Use a dedicated, restricted user account
- Monitor all system activity
- Never run on production systems

**USE AT YOUR OWN RISK!**

## Auto-Update Feature

The bot includes automatic update functionality that:

- **Monitors for updates**: Checks the main branch every 5 minutes for new commits
- **Auto-restarts**: Automatically pulls changes, rebuilds, and restarts when updates are detected
- **Zero-downtime updates**: Seamlessly updates the bot without manual intervention

### How it works:

1. The bot periodically fetches from `origin/main`
2. Compares current commit hash with remote commit hash
3. If changes are detected:
   - Pulls the latest changes
   - Rebuilds the application with `cargo build --release`
   - Restarts the bot process

### Service Installation (Recommended)

For production use, install as a systemd service:

```bash
# Copy the service file
sudo cp telegram-claude-yolo-bot.service /etc/systemd/system/

# Reload systemd and enable the service
sudo systemctl daemon-reload
sudo systemctl enable telegram-claude-yolo-bot
sudo systemctl start telegram-claude-yolo-bot

# Check status
sudo systemctl status telegram-claude-yolo-bot
```

This ensures the bot automatically starts on boot and restarts on crashes or updates.

## Disclaimer

This software is provided "as is" without warranty. The authors are not responsible for any damage or security breaches that may occur from using this software. Use at your own risk and always follow security best practices.
