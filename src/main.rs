use clap::Parser;
use libc::memcpy;
use log::{debug, error, info};
use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc, Mutex,
    },
};

use crate::{
    capture::Capture,
    mjpeg::Mjpeg,
    utils::ioctl::{
        check_support_video_streaming, close_device, format_info, init_fmt, open_device,
        read_device_capability, Buffer,
    },
};
use lazy_static::lazy_static;

mod capture;
mod error;
mod mjpeg;
mod utils;

pub struct Image {
    pub idx: usize,
    pub data: Vec<u8>,
}
impl Image {
    fn new(idx: usize, data: Vec<u8>) -> Self {
        Image { idx, data }
    }
}

lazy_static! {
    static ref CURRENT_IMAGE: Arc<Mutex<Image>> = Arc::new(Mutex::new(Image::new(0, vec![])));
    static ref IMAGE_IDX: AtomicUsize = AtomicUsize::new(0);
    static ref TOKEN_CANCELATION: Arc<Mutex<tokio_util::sync::CancellationToken>> =
        Arc::new(Mutex::new(tokio_util::sync::CancellationToken::new()));
}

#[derive(Parser, Debug)]
#[clap(version, about = "Video Capture V4L2")]
struct Args {
    /// Path to device (default /dev/video0)
    #[clap(long, default_value_t = String::from("/dev/video0"))]
    path: String,
    /// Width picture (default 640)
    #[clap(long, default_value_t = 640)]
    width: i32,
    /// Height picture (default 480)
    #[clap(long, default_value_t = 480)]
    height: i32,
    /// Enable/Disable http mjpeg server (disable)
    #[clap(long, default_value_t = String::from("disable"))]
    mjpeg: String,
    #[clap(long, default_value_t = 8000)]
    port: u16,
}

extern "C" fn handler(buffer: Buffer) {
    /* Данные из буфера обязательно нужно копировать, чтобы перенести в вектор например,
     * так как буферы освобождаются системой
     */
    let dest = unsafe {
        let dest = libc::malloc(buffer.len as usize);
        memcpy(dest, buffer.start, buffer.len as usize);
        dest
    };
    let data =
        unsafe { Vec::from_raw_parts(dest as *mut u8, buffer.len as usize, buffer.len as usize) };
    debug!("Received picture size = {}", data.len());
    let idx = IMAGE_IDX.load(std::sync::atomic::Ordering::SeqCst) + 1;
    *CURRENT_IMAGE.lock().unwrap() = Image::new(idx, data);
    IMAGE_IDX.store(idx, std::sync::atomic::Ordering::SeqCst);
}

fn main() {
    let args = Args::parse();
    env_logger::init();
    let port = args.port;
    if args.mjpeg == "enable" {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Unable to start the tokio runtime");
            rt.block_on(async {
                info!("Started mjpeg server");
                match Mjpeg::new(format!("0.0.0.0:{}", port).parse().unwrap()).await {
                    Ok(server) => {
                        if let Err(err) = server.run().await {
                            error!("{}", err);
                        }
                    }
                    Err(err) => error!("{}", err),
                }
                info!("Stopped mjpeg server");
            });
        });
    }

    let interrupt = Arc::new(AtomicBool::new(false));
    let interrupt2 = interrupt.clone();
    let token = { TOKEN_CANCELATION.lock().unwrap().clone() };
    ctrlc::set_handler(move || {
        info!("Received signal SIGINT. Stopping app");
        interrupt2.store(true, std::sync::atomic::Ordering::Relaxed);
        token.cancel();
    })
    .expect("Error settings SIGINT handler");
    let path = CString::new(args.path.as_bytes()).expect("Invalid path to device");
    match open_device(path.as_ptr()) {
        Ok(fd) => {
            info!("Device [{}] opened successfully", args.path);
            if let Some(caps) = read_device_capability(fd) {
                info!("{}", caps);
                if check_support_video_streaming(caps.get_device_caps()) {
                    info!("The device supports streaming. We're starting to stream");
                    format_info(fd);
                    match init_fmt(fd, args.width, args.height) {
                        Err(err) => error!("{}", err),
                        Ok(_) => {
                            let capture = Capture::new();
                            capture.run(fd, interrupt, handler);
                        }
                    }
                }
            }
            match close_device(fd) {
                Ok(_) => info!("Device closed successfully"),
                Err(err) => error!("Error closing device. {}", err),
            }
        }
        Err(err) => error!("Could not open device. {}: {}", err, args.path),
    }
}
