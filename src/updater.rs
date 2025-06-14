use anyhow::Result;
use log::{error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

#[derive(Deserialize)]
struct CrateInfo {
    #[serde(rename = "crate")]
    krate: InnerCrate,
}

#[derive(Deserialize)]
struct InnerCrate {
    max_version: String,
}

pub struct AutoUpdater {
    update_interval: Duration,
    client: Client,
}

impl AutoUpdater {
    pub fn new(update_interval_minutes: u64) -> Self {
        Self {
            update_interval: Duration::from_secs(update_interval_minutes * 60),
            client: Client::new(),
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
                Ok(Some(version)) => {
                    info!("New version {} detected! Updating...", version);
                    if let Err(e) = self.install_latest().await {
                        error!("Failed to install latest version: {}", e);
                        continue;
                    }
                    if let Err(e) = self.restart_application().await {
                        error!("Failed to restart application: {}", e);
                    }
                }
                Ok(None) => {
                    info!(
                        "No updates available. Current version: {}",
                        env!("CARGO_PKG_VERSION")
                    );
                }
                Err(e) => {
                    warn!("Failed to check for updates: {}", e);
                }
            }
        }
    }

    async fn check_for_updates(&self) -> Result<Option<String>> {
        info!("Checking crates.io for updates...");
        let resp: CrateInfo = self
            .client
            .get("https://crates.io/api/v1/crates/telegram-claude-yolo-bot")
            .send()
            .await?
            .json()
            .await?;
        let latest_version = resp.krate.max_version;
        let current_version = env!("CARGO_PKG_VERSION");
        if latest_version != current_version {
            Ok(Some(latest_version))
        } else {
            Ok(None)
        }
    }

    async fn install_latest(&self) -> Result<()> {
        info!("Installing latest version from crates.io...");
        let mut child = Command::new("cargo")
            .args(["install", "telegram-claude-yolo-bot", "--force"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let status = child.wait().await?;
        if !status.success() {
            return Err(anyhow::anyhow!("cargo install failed"));
        }
        Ok(())
    }

    async fn restart_application(&self) -> Result<()> {
        info!("Restarting application...");
        let screen_session = std::env::var("STY").ok();

        if let Some(session_name) = screen_session {
            info!("Detected screen session: {}", session_name);
            let current_exe = std::env::current_exe()?;
            let args: Vec<String> = std::env::args().collect();
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;
            let exe_cstring = CString::new(current_exe.as_os_str().as_bytes())?;
            let mut arg_cstrings = Vec::new();
            arg_cstrings.push(exe_cstring.clone());
            for arg in &args[1..] {
                arg_cstrings.push(CString::new(arg.as_bytes())?);
            }
            let mut c_args: Vec<*const libc::c_char> =
                arg_cstrings.iter().map(|s| s.as_ptr()).collect();
            c_args.push(std::ptr::null());
            info!("Executing replacement process within screen session...");
            unsafe {
                libc::execv(exe_cstring.as_ptr(), c_args.as_ptr());
            }
            Err(anyhow::anyhow!("Failed to execute replacement process"))
        } else {
            info!("Not in screen session, using standard restart method");
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
