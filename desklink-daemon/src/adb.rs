use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub fn check_binary_exists(name: &str) -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let full = dir.join(name);
            if full.is_file() {
                return true;
            }
        }
    }
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub struct AdbManager {
    port: u16,
    enabled: bool,
}

impl AdbManager {
    pub fn new(port: u16, enabled: bool) -> Self {
        Self { port, enabled }
    }

    pub async fn setup_forwarding(&self) -> bool {
        if !self.enabled {
            info!("ADB forwarding is disabled via CLI flag.");
            return true;
        }

        if !check_binary_exists("adb") {
            warn!("'adb' command not found in PATH. Make sure Android SDK platform-tools or android-tools is installed.");
            return false;
        }

        info!("Configuring ADB reverse rule: tcp:{} -> tcp:{}", self.port, self.port);
        let reverse_arg = format!("tcp:{}", self.port);

        let output = Command::new("adb")
            .args(["reverse", &reverse_arg, &reverse_arg])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                info!("ADB Port reverse established successfully (Port {}).", self.port);
                true
            }
            Ok(out) => {
                let err_msg = String::from_utf8_lossy(&out.stderr);
                warn!("ADB reverse command returned: {}", err_msg.trim());
                false
            }
            Err(e) => {
                warn!("Failed to execute 'adb reverse': {}", e);
                false
            }
        }
    }

    pub async fn start_watch_loop(&self) {
        if !self.enabled {
            return;
        }

        let port = self.port;
        tokio::spawn(async move {
            let reverse_arg = format!("tcp:{}", port);
            loop {
                sleep(Duration::from_secs(5)).await;
                let _ = Command::new("adb")
                    .args(["reverse", &reverse_arg, &reverse_arg])
                    .output();
            }
        });
    }
}
