use crate::protocol::{VideoHeader, VIDEO_HEADER_SIZE};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub encoder_choice: Option<String>,
    pub custom_pipeline: Option<String>,
    pub test_pattern: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            bitrate_kbps: 8000, // 8 Mbps optimized for high quality & cool thermals
            encoder_choice: None,
            custom_pipeline: None,
            test_pattern: false,
        }
    }
}

pub struct VideoStreamer {
    config: StreamConfig,
    child_process: Option<Child>,
    is_running: Arc<AtomicBool>,
}

impl VideoStreamer {
    pub fn new(config: StreamConfig) -> Self {
        Self {
            config,
            child_process: None,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn detect_encoder(&self) -> String {
        if let Some(ref enc) = self.config.encoder_choice {
            return enc.clone();
        }

        // Check for hardware NVENC
        if let Ok(out) = std::process::Command::new("gst-inspect-1.0").arg("nvh264enc").output() {
            if out.status.success() {
                info!("Selected NVIDIA NVENC hardware encoder (nvh264enc)");
                return format!(
                    "nvh264enc preset=p1 tune=ultra-low-latency rc-mode=cbr bframes=0 gop-size={} repeat-sequence-header=true bitrate={}",
                    self.config.fps, self.config.bitrate_kbps
                );
            }
        }

        // Check for VA-API
        if let Ok(out) = std::process::Command::new("gst-inspect-1.0").arg("vah264enc").output() {
            if out.status.success() {
                info!("Selected VA-API hardware encoder (vah264enc)");
                return format!(
                    "vah264enc bitrate={} rate-control=cbr",
                    self.config.bitrate_kbps
                );
            }
        }

        // Check for x264enc
        if let Ok(out) = std::process::Command::new("gst-inspect-1.0").arg("x264enc").output() {
            if out.status.success() {
                info!("Selected x264 software encoder with zerolatency tuning");
                return format!(
                    "x264enc tune=zerolatency speed-preset=ultrafast bframes=0 key-int-max={} bitrate={}",
                    self.config.fps, self.config.bitrate_kbps
                );
            }
        }

        info!("Selected openh264enc encoder fallback");
        format!(
            "openh264enc bitrate={} gop-size={}",
            self.config.bitrate_kbps * 1000,
            self.config.fps
        )
    }

    fn build_pipeline_args(&self) -> Vec<String> {
        if let Some(ref custom) = self.config.custom_pipeline {
            return custom.split_whitespace().map(|s| s.to_string()).collect();
        }

        let encoder_str = self.detect_encoder();
        let src_pipe = if self.config.test_pattern {
            format!(
                "videotestsrc pattern=smpte is-live=true ! video/x-raw,width={},height={},framerate={}/1",
                self.config.width, self.config.height, self.config.fps
            )
        } else if let Some((startx, starty, endx, endy)) = Self::detect_virtual_output_bounds() {
            info!("Targeting Virtual Extended Monitor at region ({},{}) -> ({},{})", startx, starty, endx, endy);
            format!(
                "ximagesrc use-damage=0 startx={} starty={} endx={} endy={} ! video/x-raw,framerate={}/1 ! videoscale ! video/x-raw,width={},height={}",
                startx, starty, endx, endy, self.config.fps, self.config.width, self.config.height
            )
        } else {
            // Check if pipewiresrc or ximagesrc is available
            let has_pipewire = std::process::Command::new("gst-inspect-1.0")
                .arg("pipewiresrc")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if has_pipewire {
                format!(
                    "pipewiresrc do-timestamp=true keepalive-time=1000 ! video/x-raw,width={},height={},framerate={}/1",
                    self.config.width, self.config.height, self.config.fps
                )
            } else {
                format!(
                    "ximagesrc use-damage=0 ! video/x-raw,framerate={}/1 ! videoscale ! video/x-raw,width={},height={}",
                    self.config.fps, self.config.width, self.config.height
                )
            }
        };

        let full_pipeline = format!(
            "{} ! videoconvert ! {} ! h264parse config-interval=1 ! video/x-h264,stream-format=byte-stream,alignment=au ! fdsink fd=1",
            src_pipe, encoder_str
        );

        full_pipeline.split_whitespace().map(|s| s.to_string()).collect()
    }

    fn detect_virtual_output_bounds() -> Option<(u32, u32, u32, u32)> {
        if let Ok(out) = std::process::Command::new("xrandr").output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if (line.starts_with("Virtual") || line.starts_with("HEADLESS") || line.contains("connected")) 
                    && !line.contains("primary") 
                    && line.contains("connected") 
                    && !line.contains("disconnected") 
                {
                    for part in line.split_whitespace() {
                        if let Some((geom, pos)) = part.split_once('+') {
                            if let Some((w_str, h_str)) = geom.split_once('x') {
                                if let (Ok(w), Ok(h)) = (w_str.parse::<u32>(), h_str.parse::<u32>()) {
                                    if let Some((x_str, y_str)) = pos.split_once('+') {
                                        if let (Ok(x), Ok(y)) = (x_str.parse::<u32>(), y_str.parse::<u32>()) {
                                            return Some((x, y, x + w - 1, y + h - 1));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn start(&mut self, sender: broadcast::Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        let has_wf_recorder = crate::adb::check_binary_exists("wf-recorder");

        let mut child = if is_wayland && has_wf_recorder && !self.config.test_pattern {
            let output_target = "Virtual-1";
            info!("Launching native Wayland screencopy streamer: wf-recorder for output '{}' (NVENC H.264 @ {} FPS, {} kbps)", output_target, self.config.fps, self.config.bitrate_kbps);
            Command::new("wf-recorder")
                .arg("-o")
                .arg(output_target)
                .arg("--codec=h264_nvenc")
                .arg("-r")
                .arg(self.config.fps.to_string())
                .arg("-p")
                .arg("preset=p1")
                .arg("-p")
                .arg("tune=ull")
                .arg("-p")
                .arg("rc=vbr")
                .arg("-p")
                .arg("pix_fmt=yuv420p")
                .arg("-p")
                .arg(format!("b={}k", self.config.bitrate_kbps))
                .arg("--muxer=h264")
                .arg("-f")
                .arg("pipe:1")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?
        } else {
            let args = self.build_pipeline_args();
            info!("Launching GStreamer pipeline process: gst-launch-1.0 {}", args.join(" "));

            Command::new("gst-launch-1.0")
                .arg("-q")
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let mut stdout = child.stdout.take().ok_or("Failed to open child stdout")?;
        let is_running = self.is_running.clone();
        is_running.store(true, Ordering::SeqCst);

        tokio::spawn(async move {
            let mut read_buf = vec![0u8; 65536];
            let mut stream_buf = Vec::with_capacity(524288);
            let mut cursor = 0usize;

            while is_running.load(Ordering::SeqCst) {
                match stdout.read(&mut read_buf).await {
                    Ok(0) => {
                        warn!("Video stream EOF reached.");
                        break;
                    }
                    Ok(n) => {
                        stream_buf.extend_from_slice(&read_buf[..n]);

                        // Parse Annex-B NALUs with sliding cursor (avoids heavy memmoves)
                        while let Some((start, end)) = Self::find_nalu_range_at(&stream_buf, cursor) {
                            let nalu_bytes = &stream_buf[start..end];
                            let payload_len = nalu_bytes.len() as u32;

                            let pts = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_micros() as u64;

                            let header = VideoHeader::new(payload_len, pts);
                            let mut packet = vec![0u8; VIDEO_HEADER_SIZE + nalu_bytes.len()];
                            let mut header_buf = [0u8; VIDEO_HEADER_SIZE];
                            header.encode(&mut header_buf);

                            packet[..VIDEO_HEADER_SIZE].copy_from_slice(&header_buf);
                            packet[VIDEO_HEADER_SIZE..].copy_from_slice(nalu_bytes);

                            let _ = sender.send(packet);

                            cursor = end;
                        }

                        // Compact buffer only when cursor is sufficiently ahead
                        if cursor > 131072 {
                            stream_buf.drain(..cursor);
                            cursor = 0;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from video stream: {:?}", e);
                        break;
                    }
                }
            }
        });

        self.child_process = Some(child);
        info!("Video streaming pipeline active and delivering low-latency NALU frames.");

        Ok(())
    }

    fn find_nalu_range_at(buf: &[u8], offset: usize) -> Option<(usize, usize)> {
        if offset + 8 > buf.len() {
            return None;
        }

        let first_start = Self::find_start_code(buf, offset)?;
        let next_start_search = if buf[first_start..].starts_with(&[0, 0, 0, 1]) {
            first_start + 4
        } else {
            first_start + 3
        };

        if next_start_search >= buf.len() {
            return None;
        }

        if let Some(second_start) = Self::find_start_code(buf, next_start_search) {
            Some((first_start, second_start))
        } else {
            None
        }
    }

    fn find_nalu_range(buf: &[u8]) -> Option<(usize, usize)> {
        Self::find_nalu_range_at(buf, 0)
    }

    fn find_start_code(buf: &[u8], offset: usize) -> Option<usize> {
        if offset + 3 > buf.len() {
            return None;
        }

        let slice = &buf[offset..];
        for i in 0..slice.len().saturating_sub(3) {
            if slice[i] == 0 && slice[i + 1] == 0 {
                if slice[i + 2] == 1 {
                    return Some(offset + i);
                } else if i + 3 < slice.len() && slice[i + 2] == 0 && slice[i + 3] == 1 {
                    return Some(offset + i);
                }
            }
        }
        None
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child_process.take() {
            info!("Stopping video stream pipeline child process...");
            let _ = child.start_kill();
            self.is_running.store(false, Ordering::SeqCst);
        }
    }
}

impl Drop for VideoStreamer {
    fn drop(&mut self) {
        self.stop();
    }
}
