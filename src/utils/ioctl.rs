use std::{ffi::CStr, fmt::Display};

use errno::errno;
use libc::{c_char, c_int, c_uchar, c_uint, c_void, free};
use log::info;

use crate::error::CapViCamError;

#[repr(C)]
pub struct Buffer {
    pub start: *mut c_void,
    pub len: c_uint,
}

#[repr(C)]
struct _V4l2FMT {
    index: c_uint,
    kind: c_uint,
    flags: c_uint,
    description: [c_uchar; 32],
    pixelformat: c_uint,
    mbus_code: c_uint,
    reserved: [c_uint; 3],
}

#[repr(C)]
struct _V4l2Capability {
    driver: [c_uchar; 16],
    card: [c_uchar; 32],
    bus_info: [c_uchar; 32],
    version: c_uint,
    capabilities: c_uint,
    device_caps: c_uint,
    reserved: [c_uint; 3],
}
#[allow(dead_code)]
pub struct V4l2Capability {
    driver: String,
    card: String,
    bus_info: String,
    version: String,
    capabilities: u32,
    device_caps: u32,
}
impl V4l2Capability {
    pub fn get_device_caps(&self) -> u32 {
        self.device_caps
    }
}

impl Display for V4l2Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Device capability [driver: {}, card: {}, bus_info: {}, version: {}]",
            self.driver, self.card, self.bus_info, self.version
        )
    }
}

extern "C" {
    fn ffi_open_device(path: *const c_char) -> c_int;
    fn ffi_close_device(fd: c_int) -> c_int;
    fn ffi_read_device_capability(fd: c_int) -> *const _V4l2Capability;
    fn ffi_format_info(fd: c_int, idx: c_uint) -> *const _V4l2FMT;
    fn ffi_support_video_streaming(device_caps: c_uint) -> c_int;
    fn ffi_create_buffers(fd: c_int, cnt: c_int) -> *mut Buffer;
    fn ffi_init_fmt(fd: c_int, width: c_int, height: c_int) -> c_int;
    fn ffi_start_streaming(fd: c_int) -> c_int;
    fn ffi_read_frame(fd: c_int, buffers: *mut Buffer, callback: extern "C" fn(Buffer)) -> c_int;
    fn ffi_stop_streaming(fd: c_int, buffers: *mut Buffer, cnt: c_int) -> c_int;
}

pub fn open_device(path: *const c_char) -> Result<i32, CapViCamError> {
    let fd = unsafe { ffi_open_device(path) };
    if fd == -1 {
        return Err(CapViCamError::from(errno().to_string()));
    }
    Ok(fd)
}
pub fn close_device(fd: c_int) -> Result<(), CapViCamError> {
    let res = unsafe { ffi_close_device(fd) };
    if res == -1 {
        return Err(CapViCamError::from(errno().to_string()));
    }
    Ok(())
}
pub fn read_device_capability(fd: c_int) -> Option<V4l2Capability> {
    let caps_raw = unsafe { ffi_read_device_capability(fd) };
    if !caps_raw.is_null() {
        unsafe {
            let driver = CStr::from_bytes_until_nul(&(*caps_raw).driver);
            let card = CStr::from_bytes_until_nul(&(*caps_raw).card);
            let bus_info = CStr::from_bytes_until_nul(&(*caps_raw).bus_info);
            if let (Ok(driver), Ok(card), Ok(bus_info)) = (driver, card, bus_info) {
                let version = format!(
                    "{}.{}.{}",
                    ((*caps_raw).version >> 16) & 0xFF,
                    ((*caps_raw).version >> 8) & 0xFF,
                    (*caps_raw).version & 0xFF
                );
                let caps = V4l2Capability {
                    driver: driver.to_string_lossy().to_string(),
                    card: card.to_string_lossy().to_string(),
                    bus_info: bus_info.to_string_lossy().to_string(),
                    version,
                    capabilities: (*caps_raw).capabilities,
                    device_caps: (*caps_raw).device_caps,
                };
                libc::free(caps_raw as *mut c_void);
                return Some(caps);
            }
            libc::free(caps_raw as *mut c_void);
            return None;
        }
    }
    None
}
pub fn check_support_video_streaming(device_caps: u32) -> bool {
    unsafe { matches!(ffi_support_video_streaming(device_caps), 1) }
}

pub fn format_info(fd: c_int) {
    unsafe {
        let mut idx = 0;
        loop {
            let fmt = ffi_format_info(fd, idx);
            if fmt.is_null() {
                break;
            }
            if let Ok(description) = CStr::from_bytes_until_nul(&(*fmt).description) {
                info!(
                    "Support format: {}",
                    format!(
                        "pixelformat = '{}{}{}{}', description = '{}'",
                        ((*fmt).pixelformat & 0xFF) as u8 as char,
                        (((*fmt).pixelformat >> 8) & 0xFF) as u8 as char,
                        (((*fmt).pixelformat >> 16) & 0xFF) as u8 as char,
                        (((*fmt).pixelformat >> 24) & 0xFF) as u8 as char,
                        description.to_string_lossy().to_string()
                    )
                );
            }
            free(fmt as *mut c_void);
            idx += 1;
        }
    }
}

pub fn init_fmt(fd: c_int, width: c_int, height: c_int) -> Result<(), CapViCamError> {
    unsafe {
        if ffi_init_fmt(fd, width, height) == -1 {
            return Err(CapViCamError::from(errno().to_string()));
        }
        Ok(())
    }
}

pub fn create_buffers(fd: c_int, cnt: c_int) -> *mut Buffer {
    unsafe { ffi_create_buffers(fd, cnt) }
}

pub fn start_streaming(fd: c_int) -> Result<(), CapViCamError> {
    unsafe {
        if ffi_start_streaming(fd) == -1 {
            return Err(CapViCamError::from(errno().to_string()));
        }
        Ok(())
    }
}
pub fn stop_streaming(fd: c_int, buffers: *mut Buffer, cnt: c_int) -> Result<(), CapViCamError> {
    unsafe {
        if ffi_stop_streaming(fd, buffers, cnt) == -1 {
            return Err(CapViCamError::from(errno().to_string()));
        }
        Ok(())
    }
}

pub fn read_frame(
    fd: c_int,
    bufers: *mut Buffer,
    callback: extern "C" fn(Buffer),
) -> Result<(), CapViCamError> {
    unsafe {
        if ffi_read_frame(fd, bufers, callback) == -1 {
            return Err(CapViCamError::from(errno().to_string()));
        }
        Ok(())
    }
}
