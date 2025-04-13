use log::info;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

use crate::{error::CapViCamError, CURRENT_IMAGE, TOKEN_CANCELATION};

static HEADER: &str = "--mjpegstream\r\nContent-Type: image/jpeg\r\nContent-Length: ";

pub struct Mjpeg {
    listener: TcpListener,
}

impl Mjpeg {
    pub async fn new(addr: SocketAddr) -> Result<Self, CapViCamError> {
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                let service = Mjpeg { listener };
                Ok(service)
            }
            Err(err) => {
                Err(CapViCamError::from(format!("{}", err)))
            }
        }
    }
    pub async fn run(&self) -> tokio::io::Result<()> {
        loop {
            let (stream, _) = self.listener.accept().await.unwrap();
            info!("New client has connected");
            let token = { TOKEN_CANCELATION.lock().unwrap().clone() };
            tokio::spawn(async move {
                tokio::select! {
                    _ = token.cancelled() => {
                        info!("Сlient has disabled the application");
                    }
                    _ = streaming(stream) => {
                        info!("Сlient has disconnected");
                    }
                }
            });
        }
    }
}

async fn streaming(mut stream: TcpStream) {
    let mut idx = 0;
    let header_resp =
        "HTTP/1.0 200 OK\r\nContent-Type: multipart/x-mixed-replace;boundary=mjpegstream\r\n\r\n";
    let _ = stream.write_all(header_resp.as_bytes()).await;
    loop {
        let curr_idx = { CURRENT_IMAGE.lock().unwrap().idx };
        if curr_idx != idx {
            idx = curr_idx;
            let data = { CURRENT_IMAGE.lock().unwrap().data.clone() };
            let header_image = format!(
                "{} {}\r\n\r\n",
                HEADER,
                data.len()
            );
            if stream.write_all(header_image.as_bytes()).await.is_err() {
                break;
            }
            if stream.write_all(&data).await.is_err() {
                break;
            }
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
