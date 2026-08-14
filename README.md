# DeskLink (Project DeskLink)
### Ultra-Low Latency USB Secondary Display System for Linux & Android
*IEEE Std 830-1998 Strict Compliance Specification v1.0.0*

**DeskLink** turns any Android Phone or Tablet into a high-performance, ultra-low latency (<8ms Glass-to-Glass) native secondary touchscreen display for Linux desktops (**Niri, Hyprland, KWin, Sway, X11**) over a standard USB cable via ADB reverse TCP socket streaming.

---

## 🌟 Key Features

- **⚡ Sub-8ms Glass-to-Glass Latency**: Direct hardware encoding & decoding pipeline with zero frame buffering and zero B-frames (`bframes=0`).
- **📐 Dynamic Resolution & Aspect Ratio Handshake**: Automatically detects native device resolution (e.g. 2K `2560x1600`, Foldable `2176x1812`, FHD `1920x1080`) and matches pixel-for-pixel with zero stretching or distortion.
- **🔌 Dynamic Virtual Display Lifecycle**: Automatically creates & enables `Virtual-1` monitor in Wayland when Android connects, and destroys/disables it upon disconnection (0% idle resource usage).
- **❄️ Low-Power Hardware Acceleration**:
  - **Intel Iris Xe VA-API**: Ultra-Low-Power LP Slice encoder (`low_power=1`, DMA-BUF Zero-Copy, ~1W power draw).
  - **NVIDIA NVENC**: Ultra-low-latency `preset=p1` / `tune=ull` hardware pipeline.
- **🖱️ Multi-Touch & Hardware Cursor**: 10-point multi-touch input serialized over `/dev/uinput` with simultaneous hardware mouse cursor and pointer tracking.
- **🖥️ GTK4 / Libadwaita GUI Control Center**: Clean modern Linux desktop control panel with server switches, resolution pickers, and live telemetry logs.
- **📦 CI/CD Auto-Versioning**: Automatic APK version incrementing on GitHub Actions (`DeskLink-v1.0.<Run_Number>.apk`).

---

## 📐 System Architecture

```
+-----------------------------------------------------------------------------------------+
|                                    LINUX HOST DAEMON                                    |
|                                                                                         |
|  [Virtual Output / Compositor] (Niri, Hyprland, KWin, Sway, X11, vkms DRM)              |
|       │                                                                                 |
|  [Hardware Encoder Engine] (Intel VA-API LP Zero-Copy / NVIDIA NVENC ull)                |
|       │                                                                                 |
|       │ Video: [0x44 (Magic) | 0x01 | Payload Length (4B) | PTS (8B) | Annex-B NALU]    |
|       ▼                                                                                 |
|  [TCP Server (127.0.0.1:9999)] ◄── ADB Reverse ──► [Android TCP Client Socket]          |
|       ▲                                                                                 |
|       │ Config: [0x03 | Width (4B) | Height (4B) | FPS (2B) | Density (2B)]             |
|       │ Touch:  [0x02 | Pointer ID (1B) | Action (1B) | X (4B) | Y (4B) | Pressure]     |
|       ▼                                                                                 |
|  [Linux Kernel /dev/uinput Multi-Touch & Pointer Device]                                |
+-----------------------------------------------------------------------------------------+
```

---

## 1. Low-Level Binary Protocol Specification

- **Transport**: Single TCP socket on port `9999` (forwarded over USB via `adb reverse tcp:9999 tcp:9999`).
- **Endianness**: Big-Endian (Network Byte Order).

### A. Video Stream Packet (Host -> Client)
| Field | Type / Size | Value / Description |
|---|---|---|
| Magic Byte | `uint8` (1B) | `0x44` (ASCII 'D') |
| Payload Type | `uint8` (1B) | `0x01` (H.264 Video NALU) |
| Payload Length | `uint32` (4B) | Byte length of NALU |
| PTS Timestamp | `uint64` (8B) | Microseconds since Unix epoch |
| Payload Data | Variable | Raw Annex B H.264 frame (`0x00 0x00 0x00 0x01`) |

### B. Device Config Handshake Packet (Client -> Host)
| Field | Type / Size | Value / Description |
|---|---|---|
| Event Type | `uint8` (1B) | `0x03` (Device Handshake) |
| Screen Width | `uint32` (4B) | Native hardware width (e.g. `2560`) |
| Screen Height | `uint32` (4B) | Native hardware height (e.g. `1600`) |
| Refresh Rate | `uint16` (2B) | Native FPS (e.g. `60` or `120`) |
| Density DPI | `uint16` (2B) | Screen DPI (e.g. `320`) |

### C. Multi-Touch Input Packet (Client -> Host)
| Field | Type / Size | Value / Description |
|---|---|---|
| Event Type | `uint8` (1B) | `0x02` (Touch Event) |
| Pointer ID | `uint8` (1B) | Touch slot index (0 - 9) |
| Touch Action | `uint8` (1B) | `0x00` = DOWN, `0x01` = MOVE, `0x02` = UP |
| Normalized X | `float32` (4B) | IEEE 754 Big-Endian (`0.0000` to `1.0000`) |
| Normalized Y | `float32` (4B) | IEEE 754 Big-Endian (`0.0000` to `1.0000`) |
| Pressure | `float32` (4B) | Float (`0.0` to `1.0`) |

---

## 2. Linux Host Daemon (`desklink-daemon`)

### Quick Start
```bash
# Start host daemon
./scripts/desklink-ctl.sh start

# Start with custom resolution and framerate
./scripts/desklink-ctl.sh start --width 2560 --height 1600 --fps 60 --bitrate 5000
```

### GUI Control Center (GTK4 / Libadwaita)
DeskLink includes a native desktop GUI control panel:
```bash
python3 desklink-gui/desklink_gui.py
```
Or launch **"DeskLink Control Center"** directly from your Application Launcher (Rofi / Wofi / GNOME / KDE).

### Automatic Background Service (Systemd)
To enable automatic background execution so you never need to open a terminal:
```bash
systemctl --user enable --now desklink
```

---

## 3. Android Client App (`desklink-android`)

### Features
- **Low-Latency MediaCodec Decoder**: Configured with `KEY_LOW_LATENCY` and `KEY_OPERATING_RATE = 120.0f` (<1.2ms decode latency).
- **Zero-Allocation Network Reader**: Real-time NALU stream reader with zero GC churn.
- **Anti-Buffer Bloat Queue**: Drops stale lagging frames to ensure real-time responsiveness.
- **Discreet Telemetry Toggle**: Tap `⚙ Stats` in the corner to show/hide FPS, latency, and bitrate overlay.
- **Auto Reconnect**: Instant handshake when USB cable is attached.

### Building & Installing APK
```bash
cd desklink-android
./gradlew assembleRelease
adb install -r app/build/outputs/apk/release/DeskLink-v*.apk
```
*Note: GitHub Actions automatically builds and publishes the latest APK under Releases/Artifacts on every commit.*

---

## 4. Diagnostics & Testing

```bash
# Run Rust unit tests for protocol and uinput
./scripts/desklink-ctl.sh test

# Run end-to-end Python client simulator
./scripts/desklink-ctl.sh simulate

# Run automated stress testing
python3 scripts/manual_stress_test.py
```

---

## License
Apache-2.0 © 2026 Pham Trung Tin (Amtia)
