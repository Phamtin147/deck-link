#[path = "../src/protocol.rs"]
mod protocol;

use protocol::{
    TouchAction, TouchEvent, VideoHeader, EVENT_TYPE_TOUCH, MAGIC_BYTE, PAYLOAD_TYPE_VIDEO,
    TOUCH_PACKET_SIZE, VIDEO_HEADER_SIZE,
};

#[test]
fn test_video_header_serialization() {
    let payload_len: u32 = 1048576; // 1 MB NALU
    let pts: u64 = 1723608000123456; // microsecond timestamp
    let header = VideoHeader::new(payload_len, pts);

    let mut buf = [0u8; VIDEO_HEADER_SIZE];
    header.encode(&mut buf);

    // Verify exact bytes
    assert_eq!(buf[0], MAGIC_BYTE); // 0x44 'D'
    assert_eq!(buf[1], PAYLOAD_TYPE_VIDEO); // 0x01

    // Big-Endian verification
    assert_eq!(&buf[2..6], &payload_len.to_be_bytes());
    assert_eq!(&buf[6..14], &pts.to_be_bytes());

    // Decode and verify roundtrip
    let decoded = VideoHeader::decode(&buf).expect("Failed to decode valid header");
    assert_eq!(decoded.magic, MAGIC_BYTE);
    assert_eq!(decoded.payload_type, PAYLOAD_TYPE_VIDEO);
    assert_eq!(decoded.payload_length, payload_len);
    assert_eq!(decoded.pts, pts);
}

#[test]
fn test_touch_event_serialization() {
    let event = TouchEvent::new(
        2, // Pointer ID 2
        TouchAction::Move,
        0.4567, // Normalized X
        0.8912, // Normalized Y
        0.75,   // Pressure
    );

    let mut buf = [0u8; TOUCH_PACKET_SIZE];
    event.encode(&mut buf);

    assert_eq!(buf[0], EVENT_TYPE_TOUCH); // 0x02
    assert_eq!(buf[1], 2); // Pointer ID
    assert_eq!(buf[2], 0x01); // TouchAction::Move

    // Decode and verify roundtrip
    let decoded = TouchEvent::decode(&buf).expect("Failed to decode valid touch event");
    assert_eq!(decoded.pointer_id, 2);
    assert_eq!(decoded.action, TouchAction::Move);
    assert!((decoded.normalized_x - 0.4567).abs() < 1e-5);
    assert!((decoded.normalized_y - 0.8912).abs() < 1e-5);
    assert!((decoded.pressure - 0.75).abs() < 1e-5);
}

#[test]
fn test_touch_clamping() {
    let event = TouchEvent::new(15, TouchAction::Down, -0.5, 1.5, 2.0);
    assert_eq!(event.pointer_id, 9); // Max 10 fingers (0-9)
    assert_eq!(event.normalized_x, 0.0);
    assert_eq!(event.normalized_y, 1.0);
    assert_eq!(event.pressure, 1.0);
}
