use anyhow::Result;
use log::{info, warn, error};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

pub struct AutoUpdater {
    update_interval: Duration,
}

impl AutoUpdater {
    pub fn new(update_interval_minutes: u64) -> Self {
        Self {
            update_interval: Duration::from_secs(update_interval_minutes * 60),
        }
    }

    pub async fn start_monitoring(&self) {
        info!("Starting auto-update monitoring with {} minute intervals", self.update_interval.as_secs() / 60);
        
        loop {
            sleep(self.update_interval).await;
            
            match self.check_for_updates().await {
                Ok(has_updates) => {
                    if has_updates {
                        info!("Updates detected! Initiating restart...");
                        if let Err(e) = self.restart_application().await {
                            error!("Failed to restart application: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to check for updates: {}", e);
                }
            }
        }
    }

    async fn check_for_updates(&self) -> Result<bool> {
        info!("Checking for updates from origin/main...");

        // Get current commit hash
        let current_commit = self.get_current_commit().await?;
        
        // Fetch latest changes
        self.fetch_origin().await?;
        
        // Get latest commit hash from origin/main
        let latest_commit = self.get_remote_commit().await?;
        
        let has_updates = current_commit != latest_commit;
        
        if has_updates {
            info!("Updates available: {} -> {}", current_commit, latest_commit);
        } else {
            info!("No updates available. Current commit: {}", current_commit);
        }
        
        Ok(has_updates)
    }

    async fn get_current_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get current commit hash"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn fetch_origin(&self) -> Result<()> {
        let output = Command::new("git")
            .args(&["fetch", "origin", "main"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to fetch from origin: {}", stderr));
        }

        Ok(())
    }

    async fn get_remote_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(&["rev-parse", "origin/main"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get remote commit hash"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn restart_application(&self) -> Result<()> {
        info!("Pulling latest changes...");
        
        // Pull latest changes
        let pull_output = Command::new("git")
            .args(&["pull", "origin", "main"])
            .output()
            .await?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            return Err(anyhow::anyhow!("Failed to pull changes: {}", stderr));
        }

        info!("Building updated application...");
        
        // Build the application
        let build_output = Command::new("cargo")
            .args(&["build", "--release"])
            .output()
            .await?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(anyhow::anyhow!("Failed to build application: {}", stderr));
        }

        info!("Restarting application...");
        
        // Restart the application using exec to replace current process
        let current_exe = std::env::current_exe()?;
        let args: Vec<String> = std::env::args().collect();
        
        Command::new(&current_exe)
            .args(&args[1..])  // Skip the program name
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Exit current process
        std::process::exit(0);
    }
}