### capvicam

Video capture from usb camera via v4l2 subsystem

#### Build:
```bash
cargo build --release
```

#### Usage:
```bash
RUST_LOG=info ./target/release/capvicam --path=/dev/video0 --width=1920 --height=1080 --mjpeg=enable --port=8008
```

#### View in browser:
```bash
http://localhost:8008
```
#### View over ffplay:
```bash
ffplay http://localhost:8008
```
