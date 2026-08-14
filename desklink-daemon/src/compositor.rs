use crate::adb::check_binary_exists;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorType {
    Hyprland,
    Niri,
    KWin,
    SwayWlroots,
    X11,
    GenericWayland,
}

pub struct CompositorManager {
    compositor_type: CompositorType,
    virtual_output_created: bool,
}

impl CompositorManager {
    pub fn new() -> Self {
        let compositor_type = Self::detect_compositor();
        info!("Detected desktop/compositor environment: {:?}", compositor_type);
        Self {
            compositor_type,
            virtual_output_created: false,
        }
    }

    fn detect_compositor() -> CompositorType {
        let xdg_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
        let wayland_display = std::env::var("WAYLAND_DISPLAY").is_ok();

        if xdg_desktop.contains("hyprland") || (check_binary_exists("hyprctl") && wayland_display) {
            CompositorType::Hyprland
        } else if xdg_desktop.contains("niri") || (check_binary_exists("niri") && wayland_display) {
            CompositorType::Niri
        } else if xdg_desktop.contains("kde") || xdg_desktop.contains("plasma") {
            CompositorType::KWin
        } else if xdg_desktop.contains("sway") {
            CompositorType::SwayWlroots
        } else if wayland_display {
            CompositorType::GenericWayland
        } else {
            CompositorType::X11
        }
    }

    pub fn create_virtual_output(&mut self, width: u32, height: u32, refresh_rate: u32) -> bool {
        match self.compositor_type {
            CompositorType::Hyprland => {
                info!("Creating Hyprland virtual output...");
                let output = Command::new("hyprctl")
                    .args(["output", "create", "virtual"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => {
                        info!("Hyprland virtual output created successfully: {}", String::from_utf8_lossy(&out.stdout).trim());
                        self.virtual_output_created = true;
                        true
                    }
                    Ok(out) => {
                        warn!("Hyprland hyprctl failed: {}", String::from_utf8_lossy(&out.stderr));
                        false
                    }
                    Err(e) => {
                        warn!("Failed to execute hyprctl: {}", e);
                        false
                    }
                }
            }
            CompositorType::Niri => {
                info!("Enabling Niri Virtual-1 output ({}x{} @ {}Hz)...", width, height, refresh_rate);
                let _ = Command::new("niri")
                    .args(["msg", "output", "Virtual-1", "on"])
                    .output();
                let mode_name = format!("{}x{}@{}.000", width, height, refresh_rate);
                let out = Command::new("niri")
                    .args(["msg", "output", "Virtual-1", "mode", &mode_name])
                    .output();
                if out.is_err() || !out.as_ref().unwrap().status.success() {
                    let _ = Command::new("niri")
                        .args(["msg", "output", "Virtual-1", "custom-mode", &width.to_string(), &height.to_string(), &refresh_rate.to_string()])
                        .output();
                }
                self.virtual_output_created = true;
                true
            }
            CompositorType::KWin => {
                info!("KWin Wayland detected. Virtual desktop/screencast managed via PipeWire portal.");
                self.virtual_output_created = true;
                true
            }
            CompositorType::SwayWlroots => {
                info!("wlroots-based compositor detected.");
                self.virtual_output_created = true;
                true
            }
            CompositorType::X11 => {
                info!("X11 environment detected. Setting up virtual mode with xrandr...");
                let mode_name = format!("DeskLink_{}x{}_{}", width, height, refresh_rate);
                let _ = Command::new("xrandr")
                    .args(["--output", "VIRTUAL-1", "--mode", &mode_name])
                    .output();
                self.virtual_output_created = true;
                true
            }
            CompositorType::GenericWayland => {
                info!("Generic Wayland environment detected. Streaming via PipeWire DMA-BUF.");
                self.virtual_output_created = true;
                true
            }
        }
    }

    pub fn destroy_virtual_output(&mut self) {
        if self.virtual_output_created {
            match self.compositor_type {
                CompositorType::Hyprland => {
                    info!("Removing Hyprland virtual output...");
                    let _ = Command::new("hyprctl")
                        .args(["output", "remove", "HEADLESS-1"])
                        .output();
                }
                CompositorType::Niri => {
                    info!("Disabling Niri Virtual-1 output (client disconnected)...");
                    let _ = Command::new("niri")
                        .args(["msg", "output", "Virtual-1", "off"])
                        .output();
                }
                CompositorType::X11 => {
                    let _ = Command::new("xrandr")
                        .args(["--output", "VIRTUAL-1", "--off"])
                        .output();
                }
                _ => {}
            }
        }
        self.virtual_output_created = false;
    }
}

impl Drop for CompositorManager {
    fn drop(&mut self) {
        self.destroy_virtual_output();
    }
}
