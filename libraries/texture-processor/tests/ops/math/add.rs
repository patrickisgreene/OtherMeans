use crate::temp_dir;
use image::{GrayImage, Luma, Rgba, RgbaImage};
use texture_processor::utils::{
    MaskSpec, buffered_binary_op, buffered_binary_op_masked, open_streamable_pair,
    stream_binary_op, stream_binary_op_masked,
};
use texture_processor::{DisplayFormat, ProcessError, TextureFormat};

fn gradient(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = Rgba([(x * 5) as u8, (y * 7) as u8, (x + y) as u8, 200]);
    }
    img
}

#[test]
fn streaming_png_add_matches_saturating_sum() {
    let a = gradient(11, 6);
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [250, 10, 100, 50];
    }

    let dir = temp_dir("streaming_png_add_matches_saturating_sum");
    let path_a = dir.join("a.png");
    let path_b = dir.join("b.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();

    let output_path = dir.join("out.png");
    let (sa, sb) = open_streamable_pair(&path_a, &path_b, TextureFormat::Png, "add")
        .unwrap()
        .unwrap();
    stream_binary_op(
        sa,
        sb,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    for (x, y, expected_px) in actual.enumerate_pixels() {
        let pa = a.get_pixel(x, y).0;
        let pb = b.get_pixel(x, y).0;
        let expected = [
            pa[0].saturating_add(pb[0]),
            pa[1].saturating_add(pb[1]),
            pa[2].saturating_add(pb[2]),
            pa[3].saturating_add(pb[3]),
        ];
        assert_eq!(expected_px.0, expected);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_add_matches_saturating_sum() {
    let a = gradient(9, 5);
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [5, 200, 60, 10];
    }

    let dir = temp_dir("streaming_tiff_add_matches_saturating_sum");
    let path_a = dir.join("a.tiff");
    let path_b = dir.join("b.tiff");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();

    let output_path = dir.join("out.tiff");
    let (sa, sb) = open_streamable_pair(&path_a, &path_b, TextureFormat::Tiff, "add")
        .unwrap()
        .unwrap();
    stream_binary_op(
        sa,
        sb,
        &output_path,
        TextureFormat::Tiff,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    for (x, y, expected_px) in actual.enumerate_pixels() {
        let pa = a.get_pixel(x, y).0;
        let pb = b.get_pixel(x, y).0;
        let expected = [
            pa[0].saturating_add(pb[0]),
            pa[1].saturating_add(pb[1]),
            pa[2].saturating_add(pb[2]),
            pa[3].saturating_add(pb[3]),
        ];
        assert_eq!(expected_px.0, expected);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_grayscale_add_matches_saturating_sum() {
    let mut a = GrayImage::new(10, 4);
    for (x, y, px) in a.enumerate_pixels_mut() {
        *px = Luma([(x * 20 + y) as u8]);
    }
    let mut b = GrayImage::new(10, 4);
    for (x, _y, px) in b.enumerate_pixels_mut() {
        *px = Luma([(200 + x) as u8]);
    }

    let dir = temp_dir("streaming_grayscale_add_matches_saturating_sum");
    let path_a = dir.join("a.png");
    let path_b = dir.join("b.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();

    let output_path = dir.join("out.png");
    let (sa, sb) = open_streamable_pair(&path_a, &path_b, TextureFormat::Png, "add")
        .unwrap()
        .unwrap();
    stream_binary_op(
        sa,
        sb,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_luma8();
    for (x, y, expected_px) in actual.enumerate_pixels() {
        let expected = a.get_pixel(x, y).0[0].saturating_add(b.get_pixel(x, y).0[0]);
        assert_eq!(expected_px.0[0], expected);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mismatched_channel_layouts_fall_back_to_buffered() {
    let rgb = image::RgbImage::new(6, 6);
    let gray = GrayImage::new(6, 6);

    let dir = temp_dir("mismatched_channel_layouts_fall_back_to_buffered");
    let path_a = dir.join("rgb.png");
    let path_b = dir.join("gray.png");
    rgb.save(&path_a).unwrap();
    gray.save(&path_b).unwrap();

    assert!(
        open_streamable_pair(&path_a, &path_b, TextureFormat::Png, "add")
            .unwrap()
            .is_none()
    );

    let output_path = dir.join("out.png");
    buffered_binary_op(
        &path_a,
        &path_b,
        &output_path,
        TextureFormat::Png,
        false,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    assert!(output_path.exists());

    std::fs::remove_dir_all(&dir).unwrap();
}

fn checkered_mask(width: u32, height: u32) -> GrayImage {
    let mut mask = GrayImage::new(width, height);
    for (x, y, px) in mask.enumerate_pixels_mut() {
        *px = Luma([if (x + y) % 3 == 0 { 0 } else { 255 }]);
    }
    mask
}

#[test]
fn streaming_png_masked_add_matches_buffered() {
    let a = gradient(11, 6);
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [250, 10, 100, 50];
    }
    let mask = checkered_mask(a.width(), a.height());

    let dir = temp_dir("streaming_png_masked_add_matches_buffered");
    let path_a = dir.join("a.png");
    let path_b = dir.join("b.png");
    let mask_path = dir.join("mask.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: true,
    };

    let streamed_path = dir.join("streamed.png");
    let (sa, sb, sm) = texture_processor::utils::open_streamable_masked_triple(
        &path_a,
        &path_b,
        &mask_path,
        TextureFormat::Png,
        "add",
    )
    .unwrap()
    .unwrap();
    stream_binary_op_masked(
        sa,
        sb,
        sm,
        mask_spec.excludes_white,
        &streamed_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let buffered_path = dir.join("buffered.png");
    buffered_binary_op_masked(
        &path_a,
        &path_b,
        mask_spec,
        &buffered_path,
        TextureFormat::Png,
        false,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_masked_add_matches_buffered() {
    let a = gradient(9, 5);
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [5, 200, 60, 10];
    }
    let mask = checkered_mask(a.width(), a.height());

    let dir = temp_dir("streaming_tiff_masked_add_matches_buffered");
    let path_a = dir.join("a.tiff");
    let path_b = dir.join("b.tiff");
    let mask_path = dir.join("mask.png");
    a.save(&path_a).unwrap();
    b.save(&path_b).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false,
    };

    let streamed_path = dir.join("streamed.tiff");
    let (sa, sb, sm) = texture_processor::utils::open_streamable_masked_triple(
        &path_a,
        &path_b,
        &mask_path,
        TextureFormat::Tiff,
        "add",
    )
    .unwrap()
    .unwrap();
    stream_binary_op_masked(
        sa,
        sb,
        sm,
        mask_spec.excludes_white,
        &streamed_path,
        TextureFormat::Tiff,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let buffered_path = dir.join("buffered.tiff");
    buffered_binary_op_masked(
        &path_a,
        &path_b,
        mask_spec,
        &buffered_path,
        TextureFormat::Tiff,
        false,
        DisplayFormat::Json,
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_passes_through_file1_unchanged() {
    let a = gradient(6, 6);
    let mut b = a.clone();
    for px in b.pixels_mut() {
        px.0 = [255, 255, 255, 255];
    }

    let mut mask = GrayImage::new(6, 6);
    for px in mask.pixels_mut() {
        *px = Luma([255]); // all white...
    }
    mask.put_pixel(3, 3, Luma([0])); // ...except one black (masked) pixel

    let dir = temp_dir("masked_pixel_passes_through_file1_unchanged");
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
    let (sa, sb, sm) = texture_processor::utils::open_streamable_masked_triple(
        &path_a,
        &path_b,
        &mask_path,
        TextureFormat::Png,
        "add",
    )
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
        "add",
        |x, y| x.saturating_add(y),
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();

    // Masked pixel: passes through `a` (file1) unchanged, not `a + b`.
    assert_eq!(*actual.get_pixel(3, 3), *a.get_pixel(3, 3));

    // Unmasked pixel: genuinely combined.
    let (pa, pb) = (a.get_pixel(0, 0).0, b.get_pixel(0, 0).0);
    let expected = Rgba([
        pa[0].saturating_add(pb[0]),
        pa[1].saturating_add(pb[1]),
        pa[2].saturating_add(pb[2]),
        pa[3].saturating_add(pb[3]),
    ]);
    assert_eq!(*actual.get_pixel(0, 0), expected);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn add_mismatched_dimensions_are_rejected() {
    let dir = temp_dir("add_mismatched_dimensions_are_rejected");
    let small = GrayImage::new(4, 4);
    let big = GrayImage::new(8, 8);
    let path_a = dir.join("small.png");
    let path_b = dir.join("big.png");
    small.save(&path_a).unwrap();
    big.save(&path_b).unwrap();

    assert!(matches!(
        open_streamable_pair(&path_a, &path_b, TextureFormat::Png, "add"),
        Err(ProcessError::InvalidInput(_))
    ));

    std::fs::remove_dir_all(&dir).unwrap();
}
