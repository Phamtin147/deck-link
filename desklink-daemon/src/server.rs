use crate::compositor::CompositorManager;
use crate::protocol::{TouchEvent, TOUCH_PACKET_SIZE};
use crate::streamer::{StreamConfig, VideoStreamer};
use crate::uinput::VirtualTouchscreen;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{error, info, warn};

pub struct DeskLinkServer {
    bind_addr: String,
    stream_config: StreamConfig,
    touch_sender: mpsc::Sender<TouchEvent>,
    touch_receiver: Arc<Mutex<mpsc::Receiver<TouchEvent>>>,
    video_sender: broadcast::Sender<Vec<u8>>,
    active_clients: Arc<AtomicUsize>,
    streamer: Arc<Mutex<Option<VideoStreamer>>>,
    compositor: Arc<Mutex<CompositorManager>>,
    uinput_width: u32,
    uinput_height: u32,
}

impl DeskLinkServer {
    pub fn new(bind_addr: String, stream_config: StreamConfig, compositor: CompositorManager) -> Self {
        let (touch_tx, touch_rx) = mpsc::channel::<TouchEvent>(256);
        let (video_tx, _) = broadcast::channel::<Vec<u8>>(16);
        let width = stream_config.width;
        let height = stream_config.height;

        Self {
            bind_addr,
            stream_config,
            touch_sender: touch_tx,
            touch_receiver: Arc::new(Mutex::new(touch_rx)),
            video_sender: video_tx,
            active_clients: Arc::new(AtomicUsize::new(0)),
            streamer: Arc::new(Mutex::new(None)),
            compositor: Arc::new(Mutex::new(compositor)),
            uinput_width: width,
            uinput_height: height,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ensure virtual display is OFF initially until a client connects
        self.compositor.lock().await.destroy_virtual_output();

        // 1. Start uinput handler task
        let touch_rx = self.touch_receiver.clone();
        let width = self.uinput_width;
        let height = self.uinput_height;

        tokio::spawn(async move {
            let mut uinput = VirtualTouchscreen::new(width, height);
            let mut rx = touch_rx.lock().await;
            while let Some(event) = rx.recv().await {
                if let Err(e) = uinput.handle_touch(&event) {
                    warn!("Error writing to uinput: {:?}", e);
                }
            }
        });

        // 2. Bind TCP listener
        let listener = TcpListener::bind(&self.bind_addr).await?;
        info!("DeskLink Host Server listening on TCP {}", self.bind_addr);

        loop {
            match listener.accept().await {
                Ok((socket, peer_addr)) => {
                    info!("DeskLink Client connected from: {}", peer_addr);
                    let count = self.active_clients.fetch_add(1, Ordering::SeqCst) + 1;

                    // Automatically enable virtual output on client connect
                    if count == 1 {
                        self.compositor.lock().await.create_virtual_output(
                            self.stream_config.width,
                            self.stream_config.height,
                            self.stream_config.fps,
                        );
                    }

                    let video_rx = self.video_sender.subscribe();
                    let touch_tx = self.touch_sender.clone();
                    let active_clients = self.active_clients.clone();
                    let streamer_mutex = self.streamer.clone();
                    let compositor_mutex = self.compositor.clone();
                    let video_sender = self.video_sender.clone();
                    let base_config = self.stream_config.clone();

                    let compositor_clone = self.compositor.clone();
                    let streamer_clone = self.streamer.clone();
                    tokio::spawn(async move {
                        Self::handle_client(
                            socket,
                            peer_addr,
                            video_rx,
                            touch_tx,
                            compositor_clone,
                            streamer_clone,
                            video_sender,
                            base_config,
                        ).await;
                        let count = active_clients.fetch_sub(1, Ordering::SeqCst) - 1;
                        info!("DeskLink Client {} disconnected. Active clients: {}", peer_addr, count);
                        if count == 0 {
                            info!("No active clients. Pausing video stream and disabling virtual display.");
                            let mut streamer_guard = streamer_mutex.lock().await;
                            if let Some(mut streamer) = streamer_guard.take() {
                                streamer.stop();
                            }
                            compositor_mutex.lock().await.destroy_virtual_output();
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting TCP connection: {:?}", e);
                }
            }
        }
    }

    async fn ensure_streamer_running_with_resolution(
        &self,
        width: u32,
        height: u32,
        fps: u32,
    ) {
        let mut streamer_guard = self.streamer.lock().await;
        let needs_restart = match *streamer_guard {
            Some(ref s) => s.config.width != width || s.config.height != height,
            None => true,
        };

        if needs_restart {
            if let Some(mut old) = streamer_guard.take() {
                info!("Restarting video streamer to match client resolution ({}x{})...", width, height);
                old.stop();
            }
            let mut cfg = self.stream_config.clone();
            cfg.width = width;
            cfg.height = height;
            cfg.fps = fps.min(60); // Cap at 60 FPS default for cool thermals
            let mut streamer = VideoStreamer::new(cfg);
            if let Err(e) = streamer.start(self.video_sender.clone()) {
                error!("Failed to start video streamer: {:?}", e);
            } else {
                *streamer_guard = Some(streamer);
            }
        }
    }

    async fn handle_client(
        socket: TcpStream,
        peer_addr: SocketAddr,
        mut video_rx: broadcast::Receiver<Vec<u8>>,
        touch_tx: mpsc::Sender<TouchEvent>,
        compositor: Arc<Mutex<CompositorManager>>,
        streamer: Arc<Mutex<Option<VideoStreamer>>>,
        video_sender: broadcast::Sender<Vec<u8>>,
        base_config: StreamConfig,
    ) {
        let (mut reader, mut writer) = socket.into_split();

        // Task 1: Stream Video packets (Host -> Client)
        let video_task = tokio::spawn(async move {
            while let Ok(packet) = video_rx.recv().await {
                if let Err(e) = writer.write_all(&packet).await {
                    warn!("Client {} write error: {:?}", peer_addr, e);
                    break;
                }
            }
        });

        // Task 2: Receive Touch and Config packets (Client -> Host)
        let compositor_clone = compositor.clone();
        let streamer_clone = streamer.clone();
        let touch_task = tokio::spawn(async move {
            let mut buf = [0u8; TOUCH_PACKET_SIZE];
            loop {
                match reader.read_exact(&mut buf).await {
                    Ok(_) => {
                        if buf[0] == crate::protocol::EVENT_TYPE_CONFIG {
                            if let Ok(config) = crate::protocol::ClientConfigEvent::decode(&buf) {
                                info!(
                                    "Received device native resolution handshake: {}x{} @ {}Hz (Aspect Ratio: {:.2}:1)",
                                    config.width,
                                    config.height,
                                    config.fps,
                                    config.width as f32 / config.height as f32
                                );
                                compositor_clone
                                    .lock()
                                    .await
                                    .create_virtual_output(config.width, config.height, config.fps);

                                // Dynamically launch/restart streamer matching client native resolution
                                let mut guard = streamer_clone.lock().await;
                                if let Some(mut old) = guard.take() {
                                    old.stop();
                                }
                                let mut new_cfg = base_config.clone();
                                new_cfg.width = config.width;
                                new_cfg.height = config.height;
                                new_cfg.fps = config.fps.min(60);
                                let mut new_streamer = VideoStreamer::new(new_cfg);
                                if let Err(e) = new_streamer.start(video_sender.clone()) {
                                    error!("Failed to start streamer for client resolution: {:?}", e);
                                } else {
                                    *guard = Some(new_streamer);
                                }
                            }
                        } else {
                            match TouchEvent::decode(&buf) {
                                Ok(event) => {
                                    if let Err(e) = touch_tx.send(event).await {
                                        warn!("Failed to dispatch touch event: {:?}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    warn!("Malformed packet received from {}: {:?}", peer_addr, e);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Connection closed or EOF
                        break;
                    }
                }
            }
        });

        // Wait for either stream to terminate
        tokio::select! {
            _ = video_task => {},
            _ = touch_task => {},
        };
    }
}
