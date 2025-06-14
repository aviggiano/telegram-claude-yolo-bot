use anyhow::Result;
use log::{error, info, warn};
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
        info!(
            "Starting auto-update monitoring with {} minute intervals",
            self.update_interval.as_secs() / 60
        );

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
            .args(["rev-parse", "HEAD"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get current commit hash"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn fetch_origin(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["fetch", "origin", "main"])
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
            .args(["rev-parse", "origin/main"])
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
            .args(["pull", "origin", "main"])
            .output()
            .await?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            return Err(anyhow::anyhow!("Failed to pull changes: {}", stderr));
        }

        info!("Building updated application...");

        // Build the application
        let build_output = Command::new("cargo")
            .args(["build", "--release"])
            .output()
            .await?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(anyhow::anyhow!("Failed to build application: {}", stderr));
        }

        info!("Restarting application in screen session...");

        // Check if we're running in a screen session
        let screen_session = std::env::var("STY").ok();

        if let Some(session_name) = screen_session {
            info!("Detected screen session: {}", session_name);
            // Use exec replacement to restart within the same screen session
            let current_exe = std::env::current_exe()?;
            let args: Vec<String> = std::env::args().collect();

            // Use nix::unistd::execv to replace the current process completely
            // This preserves the screen session and all environment
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let exe_cstring = CString::new(current_exe.as_os_str().as_bytes())?;
            let mut arg_cstrings = Vec::new();

            // First argument is the program name
            arg_cstrings.push(exe_cstring.clone());

            // Add remaining arguments (skip the original program name)
            for arg in &args[1..] {
                arg_cstrings.push(CString::new(arg.as_bytes())?);
            }

            // Convert to *const c_char
            let mut c_args: Vec<*const libc::c_char> =
                arg_cstrings.iter().map(|s| s.as_ptr()).collect();
            c_args.push(std::ptr::null()); // execv expects null-terminated array

            info!("Executing replacement process within screen session...");

            // Replace current process with new one - this preserves screen session
            unsafe {
                libc::execv(exe_cstring.as_ptr(), c_args.as_ptr());
            }

            // If we reach here, exec failed
            Err(anyhow::anyhow!("Failed to execute replacement process"))
        } else {
            info!("Not in screen session, using standard restart method");
            // Standard restart for non-screen environments
            let current_exe = std::env::current_exe()?;
            let args: Vec<String> = std::env::args().collect();

            Command::new(&current_exe)
                .args(&args[1..])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            std::process::exit(0);
        }
    }
}
