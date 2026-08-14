#!/usr/bin/env python3
"""
DeskLink Linux Host - GTK4 / Libadwaita Modern Control Center GUI
Ultra-Low Latency USB Secondary Display Manager
"""

import sys
import os
import subprocess
import threading
import time
import signal
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
gi.require_version('GLib', '2.0')
from gi.repository import Gtk, Adw, GLib, Gio

WORKSPACE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DAEMON_BIN = os.path.join(WORKSPACE_DIR, "desklink-daemon", "target", "release", "desklink-daemon")
CTL_SCRIPT = os.path.join(WORKSPACE_DIR, "scripts", "desklink-ctl.sh")

class DeskLinkApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="com.desklink.control",
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )
        self.window = None
        self.daemon_proc = None
        self.is_monitoring = True

    def do_activate(self):
        if not self.window:
            self.window = DeskLinkWindow(self)
        self.window.present()

class DeskLinkWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="DeskLink Control Center")
        self.set_default_size(580, 720)
        self.app = app

        # Main Box
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.set_content(main_box)

        # Header Bar
        header = Adw.HeaderBar()
        main_box.append(header)

        # Scrolled Window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        main_box.append(scrolled)

        # Clamp Container for beautiful centered layout
        clamp = Adw.Clamp(maximum_size=540)
        clamp.set_margin_top(16)
        clamp.set_margin_bottom(24)
        clamp.set_margin_start(16)
        clamp.set_margin_end(16)
        scrolled.set_child(clamp)

        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        clamp.set_child(content_box)

        # 1. Status Banner Card
        self.status_card = Adw.PreferencesGroup()
        self.status_card.set_title("Connection Status")
        content_box.append(self.status_card)

        self.status_row = Adw.ActionRow()
        self.status_row.set_title("Daemon Service")
        self.status_row.set_subtitle("Checking server status...")
        
        self.status_icon = Gtk.Image.new_from_icon_name("network-idle-symbolic")
        self.status_icon.set_pixel_size(28)
        self.status_row.add_prefix(self.status_icon)

        self.power_switch = Gtk.Switch()
        self.power_switch.set_valign(Gtk.Align.CENTER)
        self.power_switch.connect("state-set", self.on_switch_toggled)
        self.status_row.add_suffix(self.power_switch)

        self.status_card.add(self.status_row)

        # Client details row
        self.client_row = Adw.ActionRow()
        self.client_row.set_title("Active Client")
        self.client_row.set_subtitle("No device connected")
        self.status_card.add(self.client_row)

        # 2. Hardware Engine & Telemetry Card
        engine_group = Adw.PreferencesGroup()
        engine_group.set_title("Hardware & Power Acceleration")
        content_box.append(engine_group)

        self.encoder_row = Adw.ActionRow()
        self.encoder_row.set_title("Hardware Video Encoder")
        self.encoder_row.set_subtitle("Intel Iris Xe VA-API (Zero-Copy DMA-BUF, ~1W)")
        icon_gpu = Gtk.Image.new_from_icon_name("video-display-symbolic")
        self.encoder_row.add_prefix(icon_gpu)
        engine_group.add(self.encoder_row)

        self.dgpu_row = Adw.ActionRow()
        self.dgpu_row.set_title("Dedicated NVIDIA GPU")
        self.dgpu_row.set_subtitle("Asleep in D3cold (0W Power, Cool Thermals)")
        icon_temp = Gtk.Image.new_from_icon_name("power-profile-balanced-symbolic")
        self.dgpu_row.add_prefix(icon_temp)
        engine_group.add(self.dgpu_row)

        # 3. Stream & Resolution Settings
        settings_group = Adw.PreferencesGroup()
        settings_group.set_title("Display & Stream Preferences")
        content_box.append(settings_group)

        # Resolution dropdown
        self.res_row = Adw.ComboRow()
        self.res_row.set_title("Resolution Mode")
        self.res_model = Gtk.StringList.new([
            "Auto-Detect (Handshake 1:1)",
            "2K UHD (2560 x 1600)",
            "Full HD (1920 x 1080)",
            "HD Ready (1280 x 800)",
            "Square Fold (2176 x 1812)"
        ])
        self.res_row.set_model(self.res_model)
        self.res_row.set_selected(0)
        settings_group.add(self.res_row)

        # Framerate dropdown
        self.fps_row = Adw.ComboRow()
        self.fps_row.set_title("Target Framerate")
        self.fps_model = Gtk.StringList.new([
            "60 FPS (Recommended - Smooth & Cool)",
            "120 FPS (Ultra Smooth)",
            "30 FPS (Ultra Low Power)"
        ])
        self.fps_row.set_model(self.fps_model)
        self.fps_row.set_selected(0)
        settings_group.add(self.fps_row)

        # Bitrate dropdown
        self.bitrate_row = Adw.ComboRow()
        self.bitrate_row.set_title("Target Bitrate")
        self.bitrate_model = Gtk.StringList.new([
            "8.0 Mbps (Crisp Quality)",
            "6.0 Mbps (Optimal Low Power)",
            "12.0 Mbps (High Quality)",
            "15.0 Mbps (Ultra Quality)"
        ])
        self.bitrate_row.set_model(self.bitrate_model)
        self.bitrate_row.set_selected(0)
        settings_group.add(self.bitrate_row)

        # 4. Live Server Logs Expander
        logs_group = Adw.PreferencesGroup()
        logs_group.set_title("Diagnostic Logs")
        content_box.append(logs_group)

        expander = Adw.ExpanderRow()
        expander.set_title("Live Daemon Output")
        expander.set_subtitle("Real-time telemetry and connection logs")
        logs_group.add(expander)

        log_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        log_box.set_margin_top(8)
        log_box.set_margin_bottom(8)
        log_box.set_margin_start(8)
        log_box.set_margin_end(8)

        log_scrolled = Gtk.ScrolledWindow()
        log_scrolled.set_min_content_height(140)
        log_scrolled.set_max_content_height(200)

        self.log_text = Gtk.TextView()
        self.log_text.set_editable(False)
        self.log_text.set_monospace(True)
        self.log_text.set_wrap_mode(Gtk.WrapMode.CHAR)
        self.log_buffer = self.log_text.get_buffer()
        log_scrolled.set_child(self.log_text)

        log_box.append(log_scrolled)
        expander.add_row(log_box)

        # Action Buttons Box
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        btn_box.set_halign(Gtk.Align.CENTER)
        content_box.append(btn_box)

        self.btn_restart = Gtk.Button(label="Restart Server")
        self.btn_restart.connect("clicked", self.on_restart_clicked)
        btn_box.append(self.btn_restart)

        self.btn_kill_virtual = Gtk.Button(label="Reset Virtual Display")
        self.btn_kill_virtual.connect("clicked", self.on_reset_virtual_clicked)
        btn_box.append(self.btn_kill_virtual)

        # Start periodic status updater thread
        self.monitor_thread = threading.Thread(target=self.status_monitor_loop, daemon=True)
        self.monitor_thread.start()

    def is_daemon_running(self):
        try:
            out = subprocess.check_output(["pgrep", "-f", "desklink-daemon"]).decode()
            return len(out.strip()) > 0
        except Exception:
            return False

    def on_switch_toggled(self, switch, state):
        if state:
            self.start_daemon()
        else:
            self.stop_daemon()
        return False

    def start_daemon(self):
        if not self.is_daemon_running():
            fps_val = 60
            if self.fps_row.get_selected() == 1:
                fps_val = 120
            elif self.fps_row.get_selected() == 2:
                fps_val = 30

            bitrate_val = 8000
            if self.bitrate_row.get_selected() == 1:
                bitrate_val = 6000
            elif self.bitrate_row.get_selected() == 2:
                bitrate_val = 12000
            elif self.bitrate_row.get_selected() == 3:
                bitrate_val = 15000

            cmd = [CTL_SCRIPT, "start", "--fps", str(fps_val), "--bitrate", str(bitrate_val)]
            threading.Thread(target=lambda: subprocess.run(cmd), daemon=True).start()

    def stop_daemon(self):
        threading.Thread(target=lambda: subprocess.run([CTL_SCRIPT, "stop"]), daemon=True).start()

    def on_restart_clicked(self, btn):
        self.stop_daemon()
        time.sleep(0.8)
        self.start_daemon()

    def on_reset_virtual_clicked(self, btn):
        subprocess.run(["niri", "msg", "output", "Virtual-1", "off"], stderr=subprocess.DEVNULL)
        self.append_log("[INFO] Virtual-1 display reset to OFF.\n")

    def append_log(self, text):
        def _update():
            end_iter = self.log_buffer.get_end_iter()
            self.log_buffer.insert(end_iter, text)
            # Limit log size
            if self.log_buffer.get_line_count() > 300:
                start = self.log_buffer.get_start_iter()
                mid = self.log_buffer.get_iter_at_line(50)
                self.log_buffer.delete(start, mid)
        GLib.idle_add(_update)

    def status_monitor_loop(self):
        while self.app.is_monitoring:
            running = self.is_daemon_running()
            connected = False
            client_ip = "No device connected"
            active_mode = "1920x1080 (Idle)"

            # Check wlr-randr for Virtual-1 status
            try:
                out = subprocess.check_output(["wlr-randr"], stderr=subprocess.DEVNULL).decode()
                if "Virtual-1" in out:
                    for line in out.splitlines():
                        if "current" in line:
                            active_mode = line.strip().replace(" (current)", "").replace(" (preferred, current)", "")
                        if "Enabled: yes" in line:
                            connected = True
            except Exception:
                pass

            def update_ui():
                if running:
                    self.power_switch.set_active(True)
                    if connected:
                        self.status_row.set_subtitle("🟢 Online & Streaming Active (Connected)")
                        self.status_icon.set_from_icon_name("emblem-ok-symbolic")
                        self.client_row.set_subtitle(f"📱 Connected | Mode: {active_mode}")
                    else:
                        self.status_row.set_subtitle("🟡 Listening on TCP Port 9999 (Waiting for Phone)")
                        self.status_icon.set_from_icon_name("network-transmit-receive-symbolic")
                        self.client_row.set_subtitle("Waiting for Android USB Connection...")
                else:
                    self.power_switch.set_active(False)
                    self.status_row.set_subtitle("⚪ Server Stopped")
                    self.status_icon.set_from_icon_name("network-offline-symbolic")
                    self.client_row.set_subtitle("Daemon offline")

            GLib.idle_add(update_ui)
            time.sleep(1.5)

if __name__ == "__main__":
    app = DeskLinkApp()
    app.run(sys.argv)
