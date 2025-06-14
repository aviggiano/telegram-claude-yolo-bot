use anyhow::Result;
use daemonize::Daemonize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const SERVICE_NAME: &str = "telegram-claude-yolo-bot";

pub async fn start_daemon(token: String, chat_id: i64) -> Result<()> {
    let daemon = Daemonize::new()
        .pid_file(format!("/tmp/{}.pid", SERVICE_NAME))
        .chown_pid_file(true)
        .working_directory("/tmp")
        .user("nobody")
        .group("daemon")
        .umask(0o027);

    match daemon.start() {
        Ok(_) => {
            crate::bot::start_bot(token, chat_id).await?;
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Failed to start daemon: {}", e)),
    }
}

pub fn install_daemon(token: String, chat_id: i64) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let service_content = format!(
        r#"[Unit]
Description=Telegram Claude YOLO Bot
After=network.target

[Service]
Type=simple
User=nobody
Group=daemon
ExecStart={} start --token {} --chat-id {} --daemon
Restart=always
RestartSec=10
Environment=TELEGRAM_BOT_TOKEN={}
Environment=TELEGRAM_CHAT_ID={}

[Install]
WantedBy=multi-user.target
"#,
        current_exe.display(),
        token,
        chat_id,
        token,
        chat_id
    );

    let service_path = format!("/etc/systemd/system/{}.service", SERVICE_NAME);
    
    // Check if we have permission to write to systemd directory
    if !has_sudo_access() {
        return Err(anyhow::anyhow!(
            "Root access required to install system daemon. Try running with sudo."
        ));
    }

    fs::write(&service_path, service_content)?;
    
    // Reload systemd and enable the service
    Command::new("systemctl")
        .args(&["daemon-reload"])
        .status()?;
    
    Command::new("systemctl")
        .args(&["enable", SERVICE_NAME])
        .status()?;
    
    println!("Service installed at: {}", service_path);
    println!("To start the service: sudo systemctl start {}", SERVICE_NAME);
    println!("To check status: sudo systemctl status {}", SERVICE_NAME);
    
    Ok(())
}

pub fn uninstall_daemon() -> Result<()> {
    if !has_sudo_access() {
        return Err(anyhow::anyhow!(
            "Root access required to uninstall system daemon. Try running with sudo."
        ));
    }

    // Stop and disable the service
    let _ = Command::new("systemctl")
        .args(&["stop", SERVICE_NAME])
        .status();
    
    let _ = Command::new("systemctl")
        .args(&["disable", SERVICE_NAME])
        .status();
    
    // Remove the service file
    let service_path = format!("/etc/systemd/system/{}.service", SERVICE_NAME);
    if PathBuf::from(&service_path).exists() {
        fs::remove_file(&service_path)?;
    }
    
    // Reload systemd
    Command::new("systemctl")
        .args(&["daemon-reload"])
        .status()?;
    
    Ok(())
}

pub fn show_status() {
    let output = Command::new("systemctl")
        .args(&["status", SERVICE_NAME, "--no-pager"])
        .output();
    
    match output {
        Ok(output) => {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            println!("Failed to get service status: {}", e);
            println!("Service may not be installed or systemctl is not available.");
        }
    }
}

fn has_sudo_access() -> bool {
    match Command::new("sudo").arg("-n").arg("true").status() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}