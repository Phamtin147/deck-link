use crate::protocol::{TouchAction, TouchEvent};
use nix::libc;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use tracing::{info, warn};

// Linux Input Event constants
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const SYN_REPORT: u16 = 0x00;
const BTN_TOUCH: u16 = 0x14a;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_PRESSURE: u16 = 0x18;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;
const ABS_MT_PRESSURE: u16 = 0x3a;

const BUS_USB: u16 = 0x03;

const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565;
const UI_SET_ABSBIT: libc::c_ulong = 0x40045567;
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const UI_DEV_SETUP: libc::c_ulong = 0x405c5503;
const UI_ABS_SETUP: libc::c_ulong = 0x401c5504;

const MAX_SLOTS: i32 = 10;
pub const VIRTUAL_COORD_MAX: i32 = 32767;

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UInputSetup {
    id: InputId,
    name: [libc::c_char; 80],
    ff_effects_max: u32,
}

#[repr(C)]
struct InputAbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

#[repr(C)]
struct UInputAbsSetup {
    code: u16,
    absinfo: InputAbsInfo,
}

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

pub struct VirtualTouchscreen {
    file: Option<File>,
    _width: u32,
    _height: u32,
    active_touches: [bool; MAX_SLOTS as usize],
    tracking_ids: [i32; MAX_SLOTS as usize],
    next_tracking_id: i32,
}

impl VirtualTouchscreen {
    pub fn new(width: u32, height: u32) -> Self {
        let mut device = Self {
            file: None,
            _width: width,
            _height: height,
            active_touches: [false; MAX_SLOTS as usize],
            tracking_ids: [-1; MAX_SLOTS as usize],
            next_tracking_id: 1,
        };

        match device.init_uinput() {
            Ok(f) => {
                info!("Successfully created /dev/uinput DeskLink Virtual Touchscreen ({}x{})", width, height);
                device.file = Some(f);
            }
            Err(e) => {
                warn!(
                    "Unable to open /dev/uinput ({:?}). Touch input injection requires write permissions to /dev/uinput. Run 'sudo chmod 666 /dev/uinput' or set up udev rules.",
                    e
                );
            }
        }

        device
    }

    fn init_uinput(&mut self) -> io::Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")?;

        let fd = file.as_raw_fd();

        unsafe {
            // Enable Events
            if libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as libc::c_int) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_int) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(fd, UI_SET_EVBIT, EV_ABS as libc::c_int) < 0 {
                return Err(io::Error::last_os_error());
            }

            // Enable Keys
            if libc::ioctl(fd, UI_SET_KEYBIT, BTN_TOUCH as libc::c_int) < 0 {
                return Err(io::Error::last_os_error());
            }

            // Enable ABS bits
            let abs_bits = [
                ABS_X,
                ABS_Y,
                ABS_PRESSURE,
                ABS_MT_SLOT,
                ABS_MT_TOUCH_MAJOR,
                ABS_MT_POSITION_X,
                ABS_MT_POSITION_Y,
                ABS_MT_TRACKING_ID,
                ABS_MT_PRESSURE,
            ];

            for &bit in &abs_bits {
                if libc::ioctl(fd, UI_SET_ABSBIT, bit as libc::c_int) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }

            // Setup ABS dimensions
            let abs_configs = [
                (ABS_MT_SLOT, 0, MAX_SLOTS - 1),
                (ABS_MT_TRACKING_ID, 0, 65535),
                (ABS_MT_POSITION_X, 0, VIRTUAL_COORD_MAX),
                (ABS_MT_POSITION_Y, 0, VIRTUAL_COORD_MAX),
                (ABS_MT_PRESSURE, 0, 255),
                (ABS_MT_TOUCH_MAJOR, 0, 255),
                (ABS_X, 0, VIRTUAL_COORD_MAX),
                (ABS_Y, 0, VIRTUAL_COORD_MAX),
                (ABS_PRESSURE, 0, 255),
            ];

            for (code, min, max) in abs_configs {
                let mut abs_setup = UInputAbsSetup {
                    code,
                    absinfo: InputAbsInfo {
                        value: 0,
                        minimum: min,
                        maximum: max,
                        fuzz: 0,
                        flat: 0,
                        resolution: 0,
                    },
                };
                if libc::ioctl(fd, UI_ABS_SETUP, &mut abs_setup as *mut _) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }

            // Setup Device Information
            let mut usetup = UInputSetup {
                id: InputId {
                    bustype: BUS_USB,
                    vendor: 0x1234,
                    product: 0x5678,
                    version: 1,
                },
                name: [0; 80],
                ff_effects_max: 0,
            };

            let name = b"DeskLink Virtual Touchscreen\0";
            for (i, &b) in name.iter().enumerate() {
                if i < 80 {
                    usetup.name[i] = b as libc::c_char;
                }
            }

            if libc::ioctl(fd, UI_DEV_SETUP, &mut usetup as *mut _) < 0 {
                return Err(io::Error::last_os_error());
            }

            if libc::ioctl(fd, UI_DEV_CREATE) < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(file)
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) -> io::Result<()> {
        let slot = event.pointer_id as usize;
        if slot >= MAX_SLOTS as usize {
            return Ok(());
        }

        let abs_x = (event.normalized_x * VIRTUAL_COORD_MAX as f32).round() as i32;
        let abs_y = (event.normalized_y * VIRTUAL_COORD_MAX as f32).round() as i32;
        let pressure = (event.pressure * 255.0).round() as i32;

        let mut events: Vec<InputEvent> = Vec::with_capacity(8);

        // Select slot
        events.push(Self::make_event(EV_ABS, ABS_MT_SLOT, slot as i32));

        match event.action {
            TouchAction::Down => {
                let tracking_id = self.next_tracking_id;
                self.next_tracking_id = (self.next_tracking_id + 1) % 65535;
                if self.next_tracking_id == 0 {
                    self.next_tracking_id = 1;
                }
                self.tracking_ids[slot] = tracking_id;
                self.active_touches[slot] = true;

                events.push(Self::make_event(EV_ABS, ABS_MT_TRACKING_ID, tracking_id));
                events.push(Self::make_event(EV_ABS, ABS_MT_POSITION_X, abs_x));
                events.push(Self::make_event(EV_ABS, ABS_MT_POSITION_Y, abs_y));
                events.push(Self::make_event(EV_ABS, ABS_MT_PRESSURE, pressure.max(1)));
                events.push(Self::make_event(EV_ABS, ABS_MT_TOUCH_MAJOR, 10));

                // Backwards compatibility Single-touch emulation
                events.push(Self::make_event(EV_KEY, BTN_TOUCH, 1));
                events.push(Self::make_event(EV_ABS, ABS_X, abs_x));
                events.push(Self::make_event(EV_ABS, ABS_Y, abs_y));
                events.push(Self::make_event(EV_ABS, ABS_PRESSURE, pressure.max(1)));
            }
            TouchAction::Move => {
                if self.active_touches[slot] {
                    events.push(Self::make_event(EV_ABS, ABS_MT_POSITION_X, abs_x));
                    events.push(Self::make_event(EV_ABS, ABS_MT_POSITION_Y, abs_y));
                    events.push(Self::make_event(EV_ABS, ABS_MT_PRESSURE, pressure.max(1)));

                    // Update single touch
                    events.push(Self::make_event(EV_ABS, ABS_X, abs_x));
                    events.push(Self::make_event(EV_ABS, ABS_Y, abs_y));
                    events.push(Self::make_event(EV_ABS, ABS_PRESSURE, pressure.max(1)));
                }
            }
            TouchAction::Up => {
                if self.active_touches[slot] {
                    self.active_touches[slot] = false;
                    self.tracking_ids[slot] = -1;
                    events.push(Self::make_event(EV_ABS, ABS_MT_TRACKING_ID, -1));

                    let any_active = self.active_touches.iter().any(|&active| active);
                    if !any_active {
                        events.push(Self::make_event(EV_KEY, BTN_TOUCH, 0));
                        events.push(Self::make_event(EV_ABS, ABS_PRESSURE, 0));
                    }
                }
            }
        }

        // Send SYN_REPORT
        events.push(Self::make_event(EV_SYN, SYN_REPORT, 0));

        self.write_events(&events)
    }

    fn make_event(type_: u16, code: u16, value: i32) -> InputEvent {
        InputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_,
            code,
            value,
        }
    }

    fn write_events(&mut self, events: &[InputEvent]) -> io::Result<()> {
        if let Some(ref mut file) = self.file {
            let size = std::mem::size_of::<InputEvent>();
            for ev in events {
                let slice = unsafe {
                    std::slice::from_raw_parts(ev as *const _ as *const u8, size)
                };
                file.write_all(slice)?;
            }
        }
        Ok(())
    }
}

impl Drop for VirtualTouchscreen {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let fd = file.as_raw_fd();
            unsafe {
                libc::ioctl(fd, UI_DEV_DESTROY);
            }
        }
    }
}
