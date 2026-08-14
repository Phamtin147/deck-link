# DeskLink (Project DeskLink)
### Ultra-Low Latency USB Secondary Display System for Linux & Android
*IEEE Std 830-1998 Strict Compliance Specification v1.0.0*

DeskLink enables using an Android Tablet/Phone as a high-performance, ultra-low latency (<8ms Glass-to-Glass) native secondary touchscreen display for Linux desktops (Niri, Hyprland, KWin, Sway, X11) over a standard USB cable via ADB TCP port forwarding.

---

## System Architecture

```
+--------------------------------------------------------------------------------+
|                             LINUX HOST DAEMON                                  |
|                                                                                |
|  [Virtual Output / Compositor] (Niri, Hyprland, KWin, X11, PipeWire)          |
|       │                                                                        |
|  [Zero-Copy GPU H.264 Encoder] (NVENC / VA-API / x264 zerolatency)             |
|       │                                                                        |
|       │ NALU Packets: [0x44 (Magic) | 0x01 | Len (4B) | PTS (8B) | Annex-B]    |
|       ▼                                                                        |
|  [TCP Server (127.0.0.1:9999)] ◄── ADB Forward ──► [Android Client Socket]    |
|       ▲                                                                        |
|       │ Touch Packets: [0x02 | ID (1B) | Action (1B) | X (4B) | Y (4B) | P]   |
|       ▼                                                                        |
|  [Linux Kernel /dev/uinput Multi-Touch Protocol B Device]                      |
+--------------------------------------------------------------------------------+
```

---

## 1. Low-Level Binary Protocol Specification

- **Transport**: Single TCP socket on port `9999` (wrapped over USB via `adb forward tcp:9999 tcp:9999`).
- **Endianness**: Big-Endian (Network Byte Order).

### Video Stream Packet (Host -> Client)
| Field | Type / Size | Exact Value |
|---|---|---|
| Magic Byte | `uint8` (1B) | `0x44` (ASCII 'D') |
| Payload Type | `uint8` (1B) | `0x01` (H.264 NALU) |
| Payload Length | `uint32` (4B) | Big-Endian byte length |
| PTS (Timestamp) | `uint64` (8B) | Microseconds since epoch |
| Payload Data | Variable | Raw H.264 Annex B NALU (`0x00 0x00 0x00 0x01`) |

### Multi-Touch Input Packet (Client -> Host)
| Field | Type / Size | Exact Value |
|---|---|---|
| Event Type | `uint8` (1B) | `0x02` (Multi-Touch Event) |
| Pointer ID | `uint8` (1B) | Tracking ID (0 - 9 for 10-touch) |
| Touch Action | `uint8` (1B) | `0x00` = DOWN, `0x01` = MOVE, `0x02` = UP |
| Normalized X | `float32` (4B) | IEEE 754 Big-Endian (0.0000 to 1.0000) |
| Normalized Y | `float32` (4B) | IEEE 754 Big-Endian (0.0000 to 1.0000) |
| Pressure | `float32` (4B) | Float (0.0 to 1.0) |

---

## 2. Linux Host Daemon (`desklink-daemon`)

### Quick Start
```bash
# Start daemon with auto GPU encoder and ADB tunnel setup
./scripts/desklink-ctl.sh start

# Or run directly with cargo
cargo run --release --manifest-path desklink-daemon/Cargo.toml -- [OPTIONS]
```

### CLI Options
- `--port <PORT>`: TCP Port (Default: `9999`)
- `--bind <ADDR>`: Bind address (Default: `0.0.0.0`)
- `--width <WIDTH>`: Virtual display width (Default: `1920`)
- `--height <HEIGHT>`: Virtual display height (Default: `1080`)
- `--fps <FPS>`: Frame rate (Default: `60`)
- `--bitrate <KBPS>`: CBR bitrate in kbps (Default: `15000` = 15 Mbps)
- `--encoder <NAME>`: Force GStreamer encoder element (e.g. `nvh264enc`, `vah264enc`, `x264enc`)
- `--test-pattern`: Stream synthetic test pattern for benchmarking and latency testing
- `--no-adb`: Disable automatic `adb forward` invocation

---

## 3. Android Client App (`desklink-android`)

### Features
- **Asynchronous MediaCodec Hardware Decoder**: Configured with `KEY_LOW_LATENCY` on Android 11+ for <3ms decode time.
- **Direct SurfaceView Rendering**: Bypasses Android Window Manager composition overhead.
- **10-Point Multi-Touch**: Zero-heap byte allocation event serialization.
- **Sticky Immersion Mode & Keep Screen On**: Fullscreen distraction-free experience.
- **Auto Reconnect**: Reconnects instantly when USB cable is plugged/unplugged.

### Building and Installing
Open `desklink-android/` in **Android Studio** or build with Gradle:
```bash
cd desklink-android
./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
```

---

## 4. Diagnostics & Testing

```bash
# Run unit tests
./scripts/desklink-ctl.sh test

# Run end-to-end Python client simulator
./scripts/desklink-ctl.sh simulate

# Run comprehensive manual & stress test
python3 scripts/manual_stress_test.py
```
