# Hướng Dẫn Sử Dụng DeskLink (DeskLink Quick Start & User Guide)

DeskLink biến điện thoại / tablet Android thành màn hình phụ cảm ứng độ trễ siêu thấp (<8ms) cho Linux qua cáp USB.

---

## ⚡ 1. Chuẩn Bị Trước Khi Dùng (Prerequisites)

1. **Trên Android Tablet/Phone**:
   - Bật **Tùy chọn cho nhà phát triển (Developer Options)**.
   - Bật **Gỡ lỗi qua USB (USB Debugging)**.
2. **Trên Máy tính Linux**:
   - Đảm bảo tài khoản nằm trong nhóm `input` (để dùng cảm ứng không cần sudo):
     ```bash
     sudo usermod -aG input $USER
     ```
   - Cài đặt công cụ `adb`:
     ```bash
     # Arch / CachyOS / Manjaro
     sudo pacman -S android-tools
     # Ubuntu / Debian
     sudo apt install adb
     ```

---

## 🚀 2. Các Bước Kết Nối & Sử Dụng

### Bước 1: Cài đặt App lên Android
Mở thư mục `desklink-android` bằng **Android Studio** và bấm **Run**, hoặc build nhanh qua terminal:
```bash
cd desklink-android
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

### Bước 2: Cắm cáp USB & Chạy Host Daemon trên Linux
Cắm cáp USB nối điện thoại/tablet với PC, sau đó chạy lệnh:
```bash
./scripts/desklink-ctl.sh start
```
> **Host Daemon sẽ tự động:**
> 1. Nhận diện Compositor (Niri, Hyprland, KWin, X11).
> 2. Kích hoạt GPU Hardware Encoder (NVIDIA NVENC / VA-API).
> 3. Tự cấu hình cổng `adb forward tcp:9999 tcp:9999`.
> 4. Tạo thiết bị cảm ứng ảo `/dev/uinput` cho Linux.

### Bước 3: Mở App DeskLink trên Android
- Mở app **DeskLink** trên điện thoại/tablet.
- Màn hình sẽ kết nối ngay lập tức và hiển thị màn hình phụ mượt mà ở 60 FPS.
- Cảm ứng đa điểm (Multi-touch tối đa 10 ngón) trên màn hình tablet sẽ điều khiển trực tiếp con trỏ và cử chỉ trên Linux.

---

## ⚙️ 3. Tuỳ Chọn Nâng Cao (CLI Flags)

Bạn có thể tuỳ chỉnh độ phân giải, FPS và Bitrate theo ý muốn:

| Tuỳ chọn | Lệnh ví dụ | Mục đích |
|---|---|---|
| **Độ phân giải 2K** | `./desklink-ctl.sh start --width 2560 --height 1600` | Cho Tablet màn hình 2K nét căng |
| **Màn hình 120Hz** | `./desklink-ctl.sh start --fps 120` | Độ mượt tối đa nếu Tablet hỗ trợ 120Hz |
| **Tăng Bitrate** | `./desklink-ctl.sh start --bitrate 25000` | 25 Mbps cho chất lượng đồ hoạ cao |
| **Chế độ Test Pattern** | `./desklink-ctl.sh start --test-pattern` | Chạy ảnh mẫu kiểm tra độ trễ / benchmark |

---

## 🛠️ 4. Kiểm Thử & Chẩn Đoán Lỗi (Diagnostics)

- **Chạy kiểm tra toàn diện cả hệ thống**:
  ```bash
  python3 scripts/manual_stress_test.py
  ```
- **Kiểm tra thiết bị USB đã nhận chưa**:
  ```bash
  adb devices
  ```

---

## ❓ 5. Xử Lý Sự Cố Thường Gặp (FAQ)

1. **App báo "Waiting for USB Connection..."**:
   - Kiểm tra xem bạn đã bấm *"Cho phép gỡ lỗi USB từ máy tính này"* trên màn hình điện thoại chưa.
   - Gõ `adb devices` trên Linux để kiểm tra xem thiết bị có hiện chữ `device` hay không.
2. **Không chạm được cảm ứng**:
   - Chạy lệnh `ls -la /dev/uinput` để xem quyền. Nếu báo permission denied, chạy `sudo chmod 666 /dev/uinput` hoặc thêm user vào group `input`.
3. **Rút cáp USB ra cắm lại**:
   - Daemon sẽ tự động dừng GPU khi rút cáp và tự phát lại video ngay khi cắm cáp vào mà không cần khởi động lại.
