mod distance;
mod ops;

use std::path::PathBuf;

use image::{Rgba, RgbaImage};

pub(crate) fn test_image() -> RgbaImage {
    let mut img = RgbaImage::new(17, 5);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = Rgba([
            (x * 7) as u8,
            (y * 23) as u8,
            (x + y) as u8,
            ((x * y) % 256) as u8,
        ]);
    }
    img
}

pub(crate) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "texture-processor-test-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
