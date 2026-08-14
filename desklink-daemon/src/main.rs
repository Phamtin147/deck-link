mod adb;
mod compositor;
mod protocol;
mod server;
mod streamer;
mod uinput;

use adb::AdbManager;
use clap::Parser;
use compositor::CompositorManager;
use server::DeskLinkServer;
use streamer::StreamConfig;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "desklink-daemon",
    author = "Pham Trung Tin <TinPT15>",
    version = "1.0.0",
    about = "DeskLink Ultra-Low Latency USB Secondary Display Host Daemon for Linux (IEEE 830 Strict Compliance)"
)]
struct Args {
    /// TCP Port to listen on (Default: 9999)
    #[arg(short, long, default_value_t = 9999)]
    port: u16,

    /// Bind address (Default: 0.0.0.0)
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Stream Width (Default: 1920)
    #[arg(long, default_value_t = 1920)]
    width: u32,

    /// Stream Height (Default: 1080)
    #[arg(long, default_value_t = 1080)]
    height: u32,

    /// Framerate / FPS (Default: 60)
    #[arg(long, default_value_t = 60)]
    fps: u32,

    /// Video Bitrate in kbps (Default: 15000 = 15 Mbps CBR)
    #[arg(short, long, default_value_t = 15000)]
    bitrate: u32,

    /// Force specific GStreamer encoder element (e.g. nvh264enc, vah264enc, x264enc)
    #[arg(long)]
    encoder: Option<String>,

    /// Custom GStreamer pipeline string
    #[arg(long)]
    pipeline: Option<String>,

    /// Generate synthetic test pattern (e.g. for benchmark/latency measurement)
    #[arg(long)]
    test_pattern: bool,

    /// Disable automatic ADB forward tcp:9999 tcp:9999
    #[arg(long)]
    no_adb: bool,

    /// Create virtual output on Wayland / X11
    #[arg(long, default_value_t = true)]
    virtual_output: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging / tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    info!("===========================================================");
    info!(" DeskLink Linux Host Daemon v1.0.0 (Project DeskLink)");
    info!(" Ultra-Low Latency USB Secondary Display (<8ms Target)");
    info!("===========================================================");

    // 1. Virtual Output Management
    let mut compositor = CompositorManager::new();
    if args.virtual_output {
        compositor.create_virtual_output(args.width, args.height, args.fps);
    }

    // 2. ADB Forwarding
    let adb = AdbManager::new(args.port, !args.no_adb);
    adb.setup_forwarding().await;
    adb.start_watch_loop().await;

    // 3. Configure Video Streamer
    let stream_config = StreamConfig {
        width: args.width,
        height: args.height,
        fps: args.fps,
        bitrate_kbps: args.bitrate,
        encoder_choice: args.encoder,
        custom_pipeline: args.pipeline,
        test_pattern: args.test_pattern,
    };

    // 4. Start TCP Server
    let bind_addr = format!("{}:{}", args.bind, args.port);
    let server = DeskLinkServer::new(bind_addr, stream_config);

    server.run().await?;

    Ok(())
}
