mod binary_op;
mod blur;
mod channel_source;
mod concat;
mod mask;
mod pixel_stream;
mod progress;

pub use binary_op::*;
pub use blur::*;
pub use channel_source::*;
pub use concat::*;
pub use mask::*;
pub use pixel_stream::*;
pub use progress::*;

pub fn multiply_byte(a: u8, b: u8) -> u8 {
    ((u16::from(a) * u16::from(b)) / 255) as u8
}

pub fn invert_byte(byte: u8) -> u8 {
    255 - byte
}

/// Reimplements `image::imageops::contrast`'s per-sample formula: scale the
/// sample around the midpoint by `((100 + contrast) / 100)^2`, then clamp
/// back into 0..=255.
pub fn contrast_transform(contrast: f32) -> impl Fn(u8) -> u8 + Copy {
    let percent = ((100.0 + contrast) / 100.0).powi(2);
    move |byte: u8| {
        let sample = f32::from(byte);
        let scaled = ((sample / 255.0 - 0.5) * percent + 0.5) * 255.0;
        scaled.clamp(0.0, 255.0) as u8
    }
}
