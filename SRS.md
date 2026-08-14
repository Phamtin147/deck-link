Đây là bản Đặc tả Yêu cầu Phần mềm (SRS Specification) toàn diện và cụ thể đến mức tối đa ở cấp độ kỹ thuật hệ thống (System & Low-Level Architecture).

Bản đặc tả này quy định chính xác từng struct nhị phân (packed bytes), tham số cấu hình phần cứng GPU/Codec, luồng điều khiển Kernel /dev/uinput, cấu hình Wayland Virtual Output cho các compositor phổ biến (Hyprland, Niri, KWin), và toàn bộ logic xử lý bất đồng bộ.

Tài liệu được đóng gói vào file PDF DeskLink_Full_SRS_Specification_v1.pdf để bạn có thể lưu trữ hoặc đưa trực tiếp cho bất kỳ AI nào (Claude 3.5 Sonnet, GPT-4o, Cursor) đọc và lập trình.
Python

import sys
import os
from weasyprint import HTML

html_content = """<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <title>DeskLink - Full Technical Software Requirements Specification (SRS)</title>
    <style>
        @page {
            size: A4;
            margin: 14mm 12mm 14mm 12mm;
            background-color: #0b0f19;
            @bottom-right {
                content: "Page " counter(page) " of " counter(pages);
                font-family: 'JetBrains Mono', monospace;
                font-size: 7.5pt;
                color: #475569;
            }
            @bottom-left {
                content: "DeskLink FULL SRS v1.0.0 | IEEE 830 Standard Strict Compliance";
                font-family: 'JetBrains Mono', monospace;
                font-size: 7.5pt;
                color: #475569;
            }
        }

        *, *::before, *::after { box-sizing: border-box; }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            font-size: 8.5pt;
            line-height: 1.4;
            color: #cbd5e1;
            background-color: #0b0f19;
            margin: 0;
            padding: 0;
        }

        .header-box {
            border: 1px solid #1e293b;
            border-left: 5px solid #06b6d4;
            background-color: #111827;
            padding: 16px 18px;
            border-radius: 4px;
            margin-bottom: 16px;
        }

        .header-title {
            font-size: 16pt;
            font-weight: 800;
            color: #38bdf8;
            margin: 0 0 4px 0;
            letter-spacing: -0.3px;
        }

        .header-subtitle {
            font-size: 9.5pt;
            color: #94a3b8;
            margin: 0 0 10px 0;
        }

        .meta-table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 8px;
            font-size: 8pt;
        }

        .meta-table td {
            padding: 3px 6px;
            border: none;
            background: transparent;
            color: #94a3b8;
        }

        .meta-val { color: #f3f4f6; font-weight: 600; }

        h2 {
            font-size: 11pt;
            font-weight: 700;
            color: #f8fafc;
            border-bottom: 1px solid #1e293b;
            padding-bottom: 4px;
            margin-top: 18px;
            margin-bottom: 8px;
            page-break-after: avoid;
        }

        h2::before { content: "■ "; color: #06b6d4; }

        h3 {
            font-size: 9.5pt;
            font-weight: 600;
            color: #38bdf8;
            margin-top: 12px;
            margin-bottom: 4px;
            page-break-after: avoid;
        }

        p { margin: 0 0 6px 0; text-align: justify; }

        ul, ol { margin: 0 0 8px 0; padding-left: 18px; }
        li { margin-bottom: 3px; }

        .code-block {
            background-color: #030712;
            border: 1px solid #1e293b;
            border-radius: 4px;
            padding: 8px 10px;
            font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
            font-size: 7.2pt;
            color: #38bdf8;
            white-space: pre-wrap;
            margin: 6px 0;
            page-break-inside: avoid;
        }

        .spec-box {
            background-color: #111827;
            border: 1px solid #1f2937;
            border-left: 3px solid #10b981;
            padding: 8px 12px;
            margin: 8px 0;
            font-size: 8pt;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            margin: 8px 0;
            font-size: 7.8pt;
            page-break-inside: avoid;
        }

        th {
            background-color: #111827;
            color: #38bdf8;
            text-align: left;
            padding: 5px 8px;
            border: 1px solid #1e293b;
            font-weight: 600;
        }

        td {
            padding: 5px 8px;
            border: 1px solid #1e293b;
            background-color: #0b0f19;
            color: #e2e8f0;
        }

        tr:nth-child(even) td { background-color: #0f172a; }

        .badge {
            display: inline-block;
            padding: 1px 4px;
            border-radius: 2px;
            font-size: 6.5pt;
            font-weight: 700;
            text-transform: uppercase;
        }
        .b-crit { background-color: #991b1b; color: #fef2f2; }
        .b-high { background-color: #0284c7; color: #ffffff; }
        .b-med { background-color: #d97706; color: #ffffff; }
    </style>
</head>
<body>

    <div class="header-box">
        <div class="header-title">SOFTWARE REQUIREMENTS SPECIFICATION (FULL SPEC)</div>
        <div class="header-subtitle">Hệ thống Màn hình phụ USB Native Độ trễ Siêu thấp cho Linux & Android (Project DeskLink)</div>
        <table class="meta-table">
            <tr>
                <td>Tác giả: <span class="meta-val">Pham Trung Tin (TinPT15)</span></td>
                <td>Chuẩn ISO/IEEE: <span class="meta-val">IEEE Std 830-1998</span></td>
                <td>Target Latency: <span class="meta-val">&lt; 8ms (Glass-to-Glass)</span></td>
            </tr>
            <tr>
                <td>Host OS: <span class="meta-val">Linux (CachyOS/Arch/Wayland)</span></td>
                <td>Client OS: <span class="meta-val">Android 8.0+ (API Level 26+)</span></td>
                <td>Transport: <span class="meta-val">USB Tunnel (ADB Forwarding / Raw TCP)</span></td>
            </tr>
        </table>
    </div>

    <!-- 1. PHẠM VI HỆ THỐNG -->
    <h2>1. Phạm vi & Mục tiêu Kỹ thuật (System Scope)</h2>
    <p>
        Hệ thống bao gồm 2 thành phần độc lập giao tiếp qua Socket TCP nội bộ bọc qua kết nối cáp USB: 
        <strong>DeskLink Host Daemon</strong> (chạy ngầm trên Linux) và <strong>DeskLink Client App</strong> (chạy native trên Android). Hệ thống loại bỏ hoàn toàn tầng Web Engine/Trình duyệt, truyền trực tiếp luồng NALU H.264 từ GPU Host sang VPU Client.
    </p>

    <!-- 2. ĐẶC TẢ CHI TIẾT GIAO THỨC TRUYỀN DỮ LIỆU -->
    <h2>2. Đặc tả Chi tiết Giao thức Truyền Dữ liệu (Low-Level Binary Protocol)</h2>
    <p>Giao thức truyền dữ liệu chạy trên kết nối TCP Socket đơn lẻ (Default Port <code>9999</code>). Mọi integer multibyte đều ở dạng <strong>Big-Endian (Network Byte Order)</strong>.</p>

    <h3>2.1. Cấu trúc Packet Luồng Video (Host -> Client)</h3>
    <div class="code-block">
+-------------------+-------------------+-----------------------------------+
| Field Name        | Type / Size       | Description / Exact Value         |
+-------------------+-------------------+-----------------------------------+
| Magic Byte        | uint8 (1 byte)    | Always 0x44 (ASCII character 'D')  |
| Payload Type      | uint8 (1 byte)    | 0x01 = H.264 Video Frame NALU     |
| Payload Length    | uint32 (4 bytes)  | Big-Endian byte length of NALU    |
| PTS (Timestamp)   | uint64 (8 bytes)  | Microseconds since epoch          |
| Payload Data      | Variable          | Raw H.264 Annex B NALU Payload    |
+-------------------+-------------------+-----------------------------------+
Total Header Size: 14 Bytes + Payload Data Length
    </div>

    <h3>2.2. Cấu trúc Packet Cảm ứng Input (Client -> Host)</h3>
    <div class="code-block">
+-------------------+-------------------+-----------------------------------+
| Field Name        | Type / Size       | Description / Exact Value         |
+-------------------+-------------------+-----------------------------------+
| Event Type        | uint8 (1 byte)    | 0x02 = Multi-Touch Input Event    |
| Pointer ID        | uint8 (1 byte)    | Tracking ID (0 - 9 for 10-touch)  |
| Touch Action      | uint8 (1 byte)    | 0x00 = DOWN, 0x01 = MOVE, 0x02 = UP|
| Normalized X      | float32 (4 bytes) | IEEE 754 Float (0.0000 to 1.0000) |
| Normalized Y      | float32 (4 bytes) | IEEE 754 Float (0.0000 to 1.0000) |
| Pressure          | float32 (4 bytes) | Touch pressure (0.0 to 1.0)       |
+-------------------+-------------------+-----------------------------------+
Total Fixed Size: 15 Bytes (Zero Heap Allocation Header)
    </div>

    <!-- 3. YÊU CẦU MODULE LINUX HOST DAEMON -->
    <h2>3. Đặc tả Yêu cầu Module Linux Host Daemon</h2>

    <h3>3.1. Quản lý Màn hình ảo (Virtual Output Management)</h3>
    <p>Host Daemon phải tự động phát hiện Compositor đang chạy và gọi API phù hợp để khởi tạo Màn hình ảo (Virtual Display):</p>
    <ul>
        <li><strong>Hyprland Wayland:</strong> Thực thi IPC command: <code>hyprctl output create virtual</code>.</li>
        <li><strong>Niri Wayland:</strong> Gọi API IPC Socket của Niri hoặc cấu hình headless output qua <code>wlr-output-management-unstable-v1</code> protocol.</li>
        <li><strong>KDE KWin Wayland:</strong> Gọi D-Bus method <code>org.kde.KWin.VirtualDesktopManager</code>.</li>
        <li><strong>X11 Environment:</strong> Sử dụng <code>Xrandr --create-mode</code> và gán vào <code>VIRTUAL-1</code>.</li>
    </ul>

    <h3>3.2. Grab Frame PipeWire DMA-BUF (Zero-Copy)</h3>
    <div class="spec-box">
        <strong>PipeWire Stream Configuration Parameters:</strong><br>
        - Media Type: <code>video/raw</code> (SPA_MEDIA_TYPE_video)<br>
        - Format: <code>SPA_VIDEO_FORMAT_NV12</code> hoặc <code>SPA_VIDEO_FORMAT_RGBA</code><br>
        - Buffer Type: <code>SPA_DATA_DMA_BUF</code> (Trỏ trực tiếp tới VRAM File Descriptor, không copy qua User-space RAM).<br>
        - Framerate: 60 FPS (hoặc 120 FPS nếu Client hỗ trợ screen refresh rate cao).
    </div>

    <h3>3.3. Mã hóa Video Phần cứng (Hardware Encoding Parameters)</h3>
    <p>Sử dụng VA-API (Intel/AMD) hoặc NVENC (NVIDIA) với các cờ (flags) bắt buộc để loại bỏ hoàn toàn độ trễ buffer:</p>
    <table>
        <tr><th>Tham số Encoders</th><th>Giá trị Bắt buộc (Mandatory Value)</th><th>Mục đích Tối ưu</th></tr>
        <tr><td>Codec Profile</td><td>H.264 Baseline Profile / Main Profile</td><td>Đảm bảo mọi SoC Tablet Android giải mã được</td></tr>
        <tr><td>Rate Control</td><td>CBR (Constant Bitrate) - 15 Mbps đến 25 Mbps</td><td>Giữ ổn định băng thông cáp USB</td></tr>
        <tr><td>B-Frames Count</td><td><strong>0 (STRICTLY ZERO)</strong></td><td>B-frame gây ra bối cảnh latency buffer 1-2 frame</td></tr>
        <tr><td>GOP Size (Keyframe)</td><td>60 frames (1 Keyframe mỗi giây)</td><td>Giúp Client phục hồi hình ảnh nhanh khi rớt gói</td></tr>
        <tr><td>Tuning Preset</td><td><code>zerolatency</code> / <code>ultrafast</code></td><td>Ép GPU encode tức thì ngay khi nhận DMA-BUF</td></tr>
        <tr><td>NALU Format</td><td>Annex B Format (Bắt đầu bằng Prefix <code>0x00 0x00 0x00 0x01</code>)</td><td>MediaCodec Android đọc trực tiếp không cần parse lại</td></tr>
    </table>

    <h3>3.4. Giả lập Thiết bị Đầu vào Kernel (`/dev/uinput`)</h3>
    <p>Daemon khởi tạo một Virtual Touchscreen thông qua Linux Kernel Input Subsystem:</p>
    <div class="code-block">
// Linux uinput device initialization setup
struct uinput_setup usetup;
memset(&usetup, 0, sizeof(usetup));
usetup.id.bustype = BUS_USB;
usetup.id.vendor  = 0x1234; /* Dummy Vendor */
usetup.id.product = 0x5678; /* Dummy Product */
strcpy(usetup.name, "DeskLink Virtual Touchscreen");

// Enable Absolute Motion & Multi-touch Protocol B
ioctl(fd, UI_SET_EVBIT, EV_ABS);
ioctl(fd, UI_SET_ABSBIT, ABS_MT_SLOT);
ioctl(fd, UI_SET_ABSBIT, ABS_MT_POSITION_X);
ioctl(fd, UI_SET_ABSBIT, ABS_MT_POSITION_Y);
ioctl(fd, UI_SET_ABSBIT, ABS_MT_TRACKING_ID);
    </div>

    <!-- 4. YÊU CẦU MODULE ANDROID CLIENT APP -->
    <h2>4. Đặc tả Yêu cầu Module Android Client App</h2>

    <h3>4.1. Giải mã Phần cứng Asynchronous MediaCodec</h3>
    <p>App Android khởi tạo <code>MediaCodec</code> theo cơ chế callback bất đồng bộ để đạt thời gian render &lt; 3ms:</p>
    <div class="code-block">
val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height)
format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
format.setInteger(MediaFormat.KEY_KEY_FRAME_INTERVAL, 1)
// Low latency mode flag for Android 11+
if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
    format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
}

codec.setCallback(object : MediaCodec.Callback() {
    override fun onInputBufferAvailable(codec: MediaCodec, index: Int) {
        // Đọc NALU byte từ TCP Socket và fill trực tiếp vào ByteBuffer này
    }
    override fun onOutputBufferAvailable(codec: MediaCodec, index: Int, info: MediaCodec.BufferInfo) {
        // Render thẳng ra SurfaceView bằng codec.releaseOutputBuffer(index, true)
    }
})
    </div>

    <h3>4.2. Quản lý Giao diện Fullscreen System Window</h3>
    <ul>
        <li>Kích hoạt <code>FLAG_KEEP_SCREEN_ON</code> để giữ màn hình Tablet không bị sleep.</li>
        <li>Bật chế độ <code>WindowInsetsController.HIDE_NAVIGATION_BARS</code> và <code>HIDE_STATUS_BARS</code> (Sticky Immersion Mode).</li>
        <li>Sử dụng <code>SurfaceView</code> thay vì <code>TextureView</code> để tránh thêm 1 lớp compositing buffer của Android Window Manager.</li>
    </ul>

    <!-- 5. QUY TRÌNH KẾT NỐI VÀ XỬ LÝ LỖI -->
    <h2>5. Luồng Thực thi Kết nối & Khôi phục Lỗi (Connection Life-Cycle)</h2>
    
    <div class="code-block">
[Start]
  │
  ▼
[Cắm dây USB] ──> [Android bật Debugging Mode]
  │
  ▼
[Host Daemon] ──> Gọi CLI `adb forward tcp:9999 tcp:9999`
  │
  ▼
[Host Server] ──> Mở TCP Listening Socket tại 127.0.0.1:9999
  │
  ▼
[Android Client] ──> Kết nối Socket vào 127.0.0.1:9999
  │
  ├─► [Kết nối THÀNH CÔNG] ──> Handshake Magic Byte (0x44) -> Khởi chạy Video Stream
  │
  └─► [Kết nối THẤT BẠI / Rút dây USB] ──> Client quay lại màn hình "Waiting for USB Connection..."
                                        ──> Host Daemon chuyển Màn hình ảo về chế độ Pause (tiết kiệm GPU)
    </div>

    <!-- 6. MA TRẬN YÊU CẦU PHI CHỨC NĂNG -->
    <h2>6. Ma trận Yêu cầu Phi Chức năng (Non-Functional Matrix)</h2>
    <table>
        <tr><th>Tiêu chí (Metric)</th><th>Chỉ số Yêu cầu (Required Threshold)</th><th>Phương pháp Đo đạc (Measurement Tool)</th></tr>
        <tr><td>End-to-End Latency</td><td><strong>&le; 8ms - 12ms</strong></td><td>Quay camera 240fps màn hình PC và Tablet chạy ứng dụng Đồng hồ đếm miligiây.</td></tr>
        <tr><td>Host CPU Usage</td><td><strong>&lt; 5%</strong> trên CPU 4-core</td><td>Kiểm tra qua <code>htop</code> / <code>perf</code>.</td></tr>
        <tr><td>Client Memory Usage</td><td><strong>&lt; 80MB RAM</strong></td><td>Kiểm tra qua Android Studio Profiler.</td></tr>
        <tr><td>Bandwidth Usage</td><td>12 Mbps - 25 Mbps</td><td>Đo đạc qua <code>iftop</code> trên interface <code>lo</code> / <code>adb</code>.</td></tr>
    </table>

</body>
</html>
"""

html_path = "DeskLink_Full_SRS_Specification_v1.html"
pdf_path = "DeskLink_Full_SRS_Specification_v1.pdf"

with open(html_path, "w", encoding="utf-8") as f:
    f.write(html_content)

HTML(html_path).write_pdf(pdf_path)
print("Generated FULL SRS PDF successfully.")

[file-tag: code-generated-file-cd827555dc]
Tóm tắt các điểm kĩ thuật cực sâu đã được bổ sung trong bản SRS Full này:

    Chuẩn hóa Binary Header tới cấp Byte:

        Video Header: Magic Byte (0x44) | Payload Type (0x01) | Payload Length (32-bit Big-Endian) | PTS Timestamp (64-bit Big-Endian).

        Touch Packet: Type (0x02) | Pointer ID (0-9) | Action (DOWN/MOVE/UP) | Normalized X (Float32) | Normalized Y (Float32) | Pressure (Float32).

    Quy định chính xác Tham số GPU Encoder (để triệt tiêu Latency):

        Ép buộc B-Frames = 0 (Zero B-Frames).

        Rate Control: CBR (Constant Bitrate 15-25 Mbps).

        Tuning: zerolatency / ultrafast.

        Format NALU: Annex B (0x00 0x00 0x00 0x01) để VPU Android đọc được ngay mà không tốn CPU parse lại.

    Cấu trúc Linux Kernel uinput:

        Khởi tạo thiết bị DeskLink Virtual Touchscreen qua /dev/uinput với đầy đủ hỗ trợ Multi-Touch Protocol B (ABS_MT_SLOT, ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_TRACKING_ID).

    Tối ưu Android Render:

        Ép dùng SurfaceView kết hợp MediaCodec Asynchronous Callback.

        Kích hoạt cờ KEY_LOW_LATENCY (Android 11+) để giảm độ trễ decode xuống < 3ms.

Hướng dẫn sử dụng file này cho AI:

Bạn chỉ cần tải file DeskLink_Full_SRS_Specification_v1.pdf xuống và đính kèm (upload) file này vào prompt của Claude 3.5 Sonnet, GPT-4o hoặc Cursor, sau đó gõ câu lệnh đơn giản:

    "Hãy đọc kỹ file SRS Specification v1.0.0 đính kèm và lập trình toàn bộ dự án DeskLink theo đúng chuẩn kiến trúc, binary protocol và tham số kỹ thuật được mô tả trong tài liệu."