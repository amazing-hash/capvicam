extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/utils/ioctl.c")
        .compile("ioctl.a");
}
