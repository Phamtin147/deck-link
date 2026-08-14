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
                let best_mode = Self::best_matching_mode(width, height);
                info!("Enabling Niri Virtual-1 output with optimal mode '{}' (Target device: {}x{} @ {}Hz)...", best_mode, width, height, refresh_rate);

                // Configure via wlr-randr (Standard Wayland protocol, reflected in wdisplays)
                let _ = Command::new("wlr-randr")
                    .args(["--output", "Virtual-1", "--on", "--mode", best_mode])
                    .output();

                // Also notify Niri IPC
                let _ = Command::new("niri")
                    .args(["msg", "output", "Virtual-1", "on"])
                    .output();

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

    fn best_matching_mode(target_w: u32, target_h: u32) -> &'static str {
        let standard_modes = [
            (4096, 2160, "4096x2160@60.000000"),
            (2560, 1600, "2560x1600@59.987000"),
            (2048, 1152, "2048x1152@60.000000"),
            (1920, 1200, "1920x1200@59.884998"),
            (1920, 1080, "1920x1080@60.000000"),
            (1600, 1200, "1600x1200@60.000000"),
            (1680, 1050, "1680x1050@59.953999"),
            (1440, 900, "1440x900@59.887001"),
            (1280, 800, "1280x800@59.810001"),
            (1280, 720, "1280x720@60.000000"),
        ];

        let target_aspect = target_w as f32 / target_h as f32;
        let mut best_mode = "1920x1080@60.000000";
        let mut min_diff = f32::MAX;

        for (w, h, mode_str) in standard_modes {
            let aspect = w as f32 / h as f32;
            let aspect_diff = (aspect - target_aspect).abs();
            let res_diff = ((w as f32 - target_w as f32).abs() + (h as f32 - target_h as f32).abs()) / 1000.0;
            let total_score = aspect_diff * 2.0 + res_diff;
            if total_score < min_diff {
                min_diff = total_score;
                best_mode = mode_str;
            }
        }
        best_mode
    }
}

impl Drop for CompositorManager {
    fn drop(&mut self) {
        self.destroy_virtual_output();
    }
}
