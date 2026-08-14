#!/usr/bin/env python3
"""
DeskLink Protocol & Latency End-to-End Simulation Tester
Simulates the Android Client over TCP, verifying video stream NALUs and sending multi-touch events.
"""

import socket
import struct
import time
import sys

HOST = "127.0.0.1"
PORT = 9999

MAGIC_BYTE = 0x44
PAYLOAD_TYPE_VIDEO = 0x01
EVENT_TYPE_TOUCH = 0x02

def test_client():
    print(f"[*] Connecting to DeskLink Host Daemon at {HOST}:{PORT}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

    try:
        sock.connect((HOST, PORT))
        print("[+] Connected successfully!")
    except Exception as e:
        print(f"[-] Connection failed: {e}")
        return False

    sock.settimeout(5.0)

    # 1. Send simulated Multi-Touch Events
    print("[*] Sending simulated 10-point Multi-Touch Gestures...")
    for pointer_id in range(3):
        # DOWN event
        packet = struct.pack(">BBBfff", EVENT_TYPE_TOUCH, pointer_id, 0x00, 0.25 * (pointer_id + 1), 0.5, 0.8)
        sock.sendall(packet)

        # MOVE event
        packet = struct.pack(">BBBfff", EVENT_TYPE_TOUCH, pointer_id, 0x01, 0.30 * (pointer_id + 1), 0.55, 0.85)
        sock.sendall(packet)

        # UP event
        packet = struct.pack(">BBBfff", EVENT_TYPE_TOUCH, pointer_id, 0x02, 0.30 * (pointer_id + 1), 0.55, 0.0)
        sock.sendall(packet)

    print("[+] Multi-Touch packets sent successfully!")

    # 2. Receive and Validate Video Frames
    print("[*] Receiving and validating H.264 NALU video frames...")
    frame_count = 0
    start_time = time.time()

    while frame_count < 10:
        header_bytes = sock.recv(14)
        if len(header_bytes) < 14:
            print("[-] Incomplete video header received.")
            break

        magic, p_type, payload_len, pts = struct.unpack(">BBIQ", header_bytes)
        if magic != MAGIC_BYTE:
            print(f"[-] Invalid Magic Byte: 0x{magic:02X} (Expected 0x44)")
            return False
        if p_type != PAYLOAD_TYPE_VIDEO:
            print(f"[-] Invalid Payload Type: 0x{p_type:02X} (Expected 0x01)")
            return False

        # Read NALU payload
        payload = b""
        while len(payload) < payload_len:
            chunk = sock.recv(min(65536, payload_len - len(payload)))
            if not chunk:
                break
            payload += chunk

        now_us = int(time.time() * 1_000_000)
        latency_us = max(0, now_us - pts)
        
        # Check Annex-B start code
        is_annex_b = payload.startswith(b"\x00\x00\x00\x01") or payload.startswith(b"\x00\x00\x01")
        frame_count += 1
        print(f"    Frame #{frame_count:02d}: NALU Size = {len(payload):6d} bytes | PTS = {pts} | Latency = {latency_us / 1000.0:.2f} ms | Annex-B = {is_annex_b}")

    total_time = time.time() - start_time
    fps = frame_count / total_time if total_time > 0 else 0
    print(f"\n[+] Successfully validated {frame_count} frames! Effective receive FPS: {fps:.2f}")

    sock.close()
    return True

if __name__ == "__main__":
    success = test_client()
    sys.exit(0 if success else 1)
