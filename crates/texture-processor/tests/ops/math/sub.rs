use crate::*;
use image::{GrayImage, Luma, Rgba, RgbaImage};
use texture_processor::DisplayFormat;
use texture_processor::TextureFormat;
use texture_processor::utils::{
    MaskSpec, open_streamable_masked_triple, open_streamable_pair, stream_binary_op,
    stream_binary_op_masked,
};

#[test]
fn streaming_png_sub_matches_saturating_difference() {
    let mut a = RgbaImage::new(11, 6);
    for (x, y, px) in a.enumerate_pixels_mut() {
        *px = Rgba([(x * 5) as u8, (y * 7) as u8, (x + y) as u8, 200]);
    }
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [250, 10, 100, 220];
    }

    let dir = temp_dir("streaming_png_sub_matches_saturating_difference");
    let path_a = dir.join("a.png");
    let path_b = dir.join("b.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();

    let output_path = dir.join("out.png");
    let (sa, sb) = open_streamable_pair(&path_a, &path_b, TextureFormat::Png, "sub")
        .unwrap()
        .unwrap();
    stream_binary_op(
        sa,
        sb,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "sub",
        |x, y| x.saturating_sub(y),
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    for (x, y, expected_px) in actual.enumerate_pixels() {
        let pa = a.get_pixel(x, y).0;
        let pb = b.get_pixel(x, y).0;
        let expected = [
            pa[0].saturating_sub(pb[0]),
            pa[1].saturating_sub(pb[1]),
            pa[2].saturating_sub(pb[2]),
            pa[3].saturating_sub(pb[3]),
        ];
        assert_eq!(expected_px.0, expected);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_passes_through_sub() {
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

    let dir = temp_dir("masked_pixel_passes_through_sub");
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
        open_streamable_masked_triple(&path_a, &path_b, &mask_path, TextureFormat::Png, "sub")
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
        "sub",
        |x, y| x.saturating_sub(y),
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();

    // Masked pixel: passes through `a` (file1) unchanged, not `a - b`.
    assert_eq!(*actual.get_pixel(3, 3), *a.get_pixel(3, 3));

    // Unmasked pixel: genuinely combined.
    let (pa, pb) = (a.get_pixel(0, 0).0, b.get_pixel(0, 0).0);
    let expected = Rgba([
        pa[0].saturating_sub(pb[0]),
        pa[1].saturating_sub(pb[1]),
        pa[2].saturating_sub(pb[2]),
        pa[3].saturating_sub(pb[3]),
    ]);
    assert_eq!(*actual.get_pixel(0, 0), expected);

    std::fs::remove_dir_all(&dir).unwrap();
}
