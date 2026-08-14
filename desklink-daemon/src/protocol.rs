use byteorder::{BigEndian, ByteOrder};
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAGIC_BYTE: u8 = 0x44; // ASCII 'D'
pub const PAYLOAD_TYPE_VIDEO: u8 = 0x01; // H.264 Video Frame NALU
pub const EVENT_TYPE_TOUCH: u8 = 0x02; // Multi-Touch Input Event

pub const VIDEO_HEADER_SIZE: usize = 14;
pub const TOUCH_PACKET_SIZE: usize = 15;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchAction {
    Down = 0x00,
    Move = 0x01,
    Up = 0x02,
}

impl TryFrom<u8> for TouchAction {
    type Error = io::Error;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0x00 => Ok(TouchAction::Down),
            0x01 => Ok(TouchAction::Move),
            0x02 => Ok(TouchAction::Up),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid TouchAction byte: 0x{:02X}", other),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoHeader {
    pub magic: u8,
    pub payload_type: u8,
    pub payload_length: u32,
    pub pts: u64, // Microseconds since epoch
}

impl VideoHeader {
    pub fn new(payload_length: u32, pts: u64) -> Self {
        Self {
            magic: MAGIC_BYTE,
            payload_type: PAYLOAD_TYPE_VIDEO,
            payload_length,
            pts,
        }
    }

    pub fn new_now(payload_length: u32) -> Self {
        let pts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        Self::new(payload_length, pts)
    }

    pub fn encode(&self, buf: &mut [u8; VIDEO_HEADER_SIZE]) {
        buf[0] = self.magic;
        buf[1] = self.payload_type;
        BigEndian::write_u32(&mut buf[2..6], self.payload_length);
        BigEndian::write_u64(&mut buf[6..14], self.pts);
    }

    pub fn decode(buf: &[u8; VIDEO_HEADER_SIZE]) -> io::Result<Self> {
        let magic = buf[0];
        if magic != MAGIC_BYTE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid magic byte: 0x{:02X}, expected 0x{:02X}", magic, MAGIC_BYTE),
            ));
        }

        let payload_type = buf[1];
        if payload_type != PAYLOAD_TYPE_VIDEO {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid payload type: 0x{:02X}, expected 0x{:02X}", payload_type, PAYLOAD_TYPE_VIDEO),
            ));
        }

        let payload_length = BigEndian::read_u32(&buf[2..6]);
        let pts = BigEndian::read_u64(&buf[6..14]);

        Ok(Self {
            magic,
            payload_type,
            payload_length,
            pts,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TouchEvent {
    pub pointer_id: u8,
    pub action: TouchAction,
    pub normalized_x: f32, // 0.0000 to 1.0000
    pub normalized_y: f32, // 0.0000 to 1.0000
    pub pressure: f32,     // 0.0 to 1.0
}

impl TouchEvent {
    pub fn new(
        pointer_id: u8,
        action: TouchAction,
        normalized_x: f32,
        normalized_y: f32,
        pressure: f32,
    ) -> Self {
        Self {
            pointer_id: pointer_id.min(9),
            action,
            normalized_x: normalized_x.clamp(0.0, 1.0),
            normalized_y: normalized_y.clamp(0.0, 1.0),
            pressure: pressure.clamp(0.0, 1.0),
        }
    }

    pub fn encode(&self, buf: &mut [u8; TOUCH_PACKET_SIZE]) {
        buf[0] = EVENT_TYPE_TOUCH;
        buf[1] = self.pointer_id;
        buf[2] = self.action as u8;
        BigEndian::write_f32(&mut buf[3..7], self.normalized_x);
        BigEndian::write_f32(&mut buf[7..11], self.normalized_y);
        BigEndian::write_f32(&mut buf[11..15], self.pressure);
    }

    pub fn decode(buf: &[u8; TOUCH_PACKET_SIZE]) -> io::Result<Self> {
        let event_type = buf[0];
        if event_type != EVENT_TYPE_TOUCH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid touch event type: 0x{:02X}", event_type),
            ));
        }

        let pointer_id = buf[1];
        let action = TouchAction::try_from(buf[2])?;
        let normalized_x = BigEndian::read_f32(&buf[3..7]);
        let normalized_y = BigEndian::read_f32(&buf[7..11]);
        let pressure = BigEndian::read_f32(&buf[11..15]);

        Ok(Self {
            pointer_id,
            action,
            normalized_x,
            normalized_y,
            pressure,
        })
    }
}

pub const EVENT_TYPE_CONFIG: u8 = 0x03; // Client Device Config Handshake
pub const CONFIG_PACKET_SIZE: usize = 15;

#[derive(Debug, Clone, PartialEq)]
pub struct ClientConfigEvent {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub density: u16,
}

impl ClientConfigEvent {
    pub fn decode(buf: &[u8; CONFIG_PACKET_SIZE]) -> io::Result<Self> {
        let event_type = buf[0];
        if event_type != EVENT_TYPE_CONFIG {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid config event type: 0x{:02X}", event_type),
            ));
        }

        let width = BigEndian::read_u32(&buf[1..5]);
        let height = BigEndian::read_u32(&buf[5..9]);
        let fps = BigEndian::read_u16(&buf[9..11]) as u32;
        let density = BigEndian::read_u16(&buf[11..13]);

        Ok(Self {
            width: width.max(320),
            height: height.max(240),
            fps: fps.clamp(30, 240),
            density,
        })
    }
}
