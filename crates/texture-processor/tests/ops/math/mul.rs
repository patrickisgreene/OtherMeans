use crate::*;
use image::{GrayImage, Luma, Rgba, RgbaImage};
use texture_processor::utils::{
    MaskSpec, multiply_byte, open_streamable_masked_triple, open_streamable_pair, stream_binary_op,
    stream_binary_op_masked,
};
use texture_processor::{DisplayFormat, TextureFormat};

#[test]
fn multiply_byte_treats_255_as_identity_and_0_as_absorbing() {
    assert_eq!(multiply_byte(200, 255), 200);
    assert_eq!(multiply_byte(200, 0), 0);
    assert_eq!(multiply_byte(255, 255), 255);
    assert_eq!(multiply_byte(128, 128), 64);
}

#[test]
fn streaming_png_mul_matches_multiply_blend() {
    let mut a = RgbaImage::new(11, 6);
    for (x, y, px) in a.enumerate_pixels_mut() {
        *px = Rgba([(x * 5) as u8, (y * 7) as u8, (x + y) as u8, 200]);
    }
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [250, 10, 100, 220];
    }

    let dir = temp_dir("streaming_png_mul_matches_multiply_blend");
    let path_a = dir.join("a.png");
    let path_b = dir.join("b.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();

    let output_path = dir.join("out.png");
    let (sa, sb) = open_streamable_pair(&path_a, &path_b, TextureFormat::Png, "mul")
        .unwrap()
        .unwrap();
    stream_binary_op(
        sa,
        sb,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "mul",
        multiply_byte,
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    for (x, y, expected_px) in actual.enumerate_pixels() {
        let pa = a.get_pixel(x, y).0;
        let pb = b.get_pixel(x, y).0;
        let expected = [
            multiply_byte(pa[0], pb[0]),
            multiply_byte(pa[1], pb[1]),
            multiply_byte(pa[2], pb[2]),
            multiply_byte(pa[3], pb[3]),
        ];
        assert_eq!(expected_px.0, expected);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_passes_through_mul() {
    let mut a = RgbaImage::new(6, 6);
    for (x, y, px) in a.enumerate_pixels_mut() {
        *px = Rgba([(x * 5) as u8, (y * 7) as u8, (x + y) as u8, 200]);
    }
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [10, 10, 10, 10];
    }

    let mut mask = GrayImage::new(6, 6);
    for px in mask.pixels_mut() {
        *px = Luma([255]); // all white...
    }
    mask.put_pixel(3, 3, Luma([0])); // ...except one black (masked) pixel

    let dir = temp_dir("masked_pixel_passes_through_mul");
    let path_a = dir.join("a.png");
    let path_b = dir.join("b.png");
    let mask_path = dir.join("mask.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false, // --black-mask: black excluded
    };

    let output_path = dir.join("out.png");
    let (sa, sb, sm) =
        open_streamable_masked_triple(&path_a, &path_b, &mask_path, TextureFormat::Png, "mul")
            .unwrap()
            .unwrap();
    stream_binary_op_masked(
        sa,
        sb,
        sm,
        mask_spec.excludes_white,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "mul",
        multiply_byte,
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();

    // Masked pixel: passes through `a` (file1) unchanged, not `a * b`.
    assert_eq!(*actual.get_pixel(3, 3), *a.get_pixel(3, 3));

    // Unmasked pixel: genuinely combined.
    let (pa, pb) = (a.get_pixel(0, 0).0, b.get_pixel(0, 0).0);
    let expected = Rgba([
        multiply_byte(pa[0], pb[0]),
        multiply_byte(pa[1], pb[1]),
        multiply_byte(pa[2], pb[2]),
        multiply_byte(pa[3], pb[3]),
    ]);
    assert_eq!(*actual.get_pixel(0, 0), expected);

    std::fs::remove_dir_all(&dir).unwrap();
}
