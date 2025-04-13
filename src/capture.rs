use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use libc::{c_void, free};
use log::{error, info};

use crate::utils::ioctl::{create_buffers, read_frame, start_streaming, stop_streaming, Buffer};

const BUFFERS_NUM: i32 = 8;
pub struct Capture {}

impl Capture {
    pub fn new() -> Self {
        Capture {}
    }
    pub fn run(&self, fd: i32, interrupt: Arc<AtomicBool>, callback: extern "C" fn(Buffer)) {
        let buffers = create_buffers(fd, BUFFERS_NUM);
        if !buffers.is_null() {
            if let Err(err) = start_streaming(fd) {
                error!("{}", err);
                interrupt.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            info!("Streaming started");
            while !interrupt.load(std::sync::atomic::Ordering::Relaxed) {
                if let Err(err) = read_frame(fd, buffers, callback) {
                    error!("{}", err);
                    break;
                }
            }
            if let Err(err) = stop_streaming(fd, buffers, BUFFERS_NUM) {
                error!("{}", err);
            }
            info!("Streaming stopped");
            unsafe {
                free(buffers as *mut c_void);
            }
            info!("Cleanup app");
        }
    }
}
