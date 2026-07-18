use crate::*;
use image::{GrayAlphaImage, GrayImage, Luma, LumaA, Rgba, RgbaImage};
use texture_processor::ops::{BlurKernel, kernel_weights};
use texture_processor::{DisplayFormat, ProcessError, TextureFormat, utils::*};

#[test]
fn blur_row_horizontal_clamps_at_edges() {
    // radius=1 kernel, hand-picked (not derived from `kernel_weights`) so
    // this test doesn't depend on that function being correct.
    let row = [10u8, 20, 30, 40];
    let weights = [0.25f32, 0.5, 0.25];

    let out = blur_row_horizontal(&row, 1, &weights);

    // x=0: taps clamp to row[0] on the left: 0.25*10 + 0.5*10 + 0.25*20 = 12.5 -> 13
    // x=1: interior: 0.25*10 + 0.5*20 + 0.25*30 = 20.0
    // x=2: interior: 0.25*20 + 0.5*30 + 0.25*40 = 30.0
    // x=3: taps clamp to row[3] on the right: 0.25*30 + 0.5*40 + 0.25*40 = 37.5 -> 38
    assert_eq!(out, vec![13, 20, 30, 38]);
}

#[test]
fn streaming_png_box_blur_matches_buffered_box_blur() {
    let img = test_image();
    let dir = temp_dir("streaming_png_box_blur_matches_buffered_box_blur");
    let source_path = dir.join("source.png");
    img.save(&source_path).unwrap();

    let weights = kernel_weights(BlurKernel::Box, 2);

    let streamed_path = dir.join("streamed.png");
    let source = open_streamable_blur_source(&source_path, TextureFormat::Png)
        .unwrap()
        .unwrap();
    stream_blur(
        source,
        &weights,
        &streamed_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let buffered_path = dir.join("buffered.png");
    buffered_blur(
        &source_path,
        &buffered_path,
        TextureFormat::Png,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_gaussian_blur_matches_buffered_gaussian_blur() {
    let img = test_image();
    let dir = temp_dir("streaming_tiff_gaussian_blur_matches_buffered_gaussian_blur");
    let source_path = dir.join("source.tiff");
    img.save(&source_path).unwrap();

    let weights = kernel_weights(BlurKernel::Gaussian, 3);

    let streamed_path = dir.join("streamed.tiff");
    let source = open_streamable_blur_source(&source_path, TextureFormat::Tiff)
        .unwrap()
        .unwrap();
    stream_blur(
        source,
        &weights,
        &streamed_path,
        TextureFormat::Tiff,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let buffered_path = dir.join("buffered.tiff");
    buffered_blur(
        &source_path,
        &buffered_path,
        TextureFormat::Tiff,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_grayscale_blur_matches_buffered_on_grayscale_channel() {
    let mut gray = GrayImage::new(12, 8);
    for (x, y, px) in gray.enumerate_pixels_mut() {
        *px = Luma([(x * 17 + y * 5) as u8]);
    }

    let dir = temp_dir("streaming_grayscale_blur_matches_buffered_on_grayscale_channel");
    let source_path = dir.join("gray.png");
    gray.save(&source_path).unwrap();

    let weights = kernel_weights(BlurKernel::Gaussian, 2);

    let streamed_path = dir.join("streamed.png");
    let source = open_streamable_blur_source(&source_path, TextureFormat::Png)
        .unwrap()
        .unwrap();
    stream_blur(
        source,
        &weights,
        &streamed_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let buffered_path = dir.join("buffered.png");
    buffered_blur(
        &source_path,
        &buffered_path,
        TextureFormat::Png,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    // Streamed output stays 1-channel grayscale; buffered normalizes to
    // RGBA8. They're not the same byte buffer, but since the convolution is
    // applied independently per channel, blurring a grayscale image's one
    // channel must equal blurring its RGBA8-expanded (R=G=B=luma) form's R
    // channel.
    let streamed = image::open(&streamed_path).unwrap().to_luma8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    for (x, y, px) in streamed.enumerate_pixels() {
        let buffered_px = buffered.get_pixel(x, y);
        assert_eq!(px.0[0], buffered_px.0[0]);
        assert_eq!(buffered_px.0[0], buffered_px.0[1]);
        assert_eq!(buffered_px.0[1], buffered_px.0[2]);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn radius_zero_is_a_no_op() {
    assert_eq!(kernel_weights(BlurKernel::Box, 0), vec![1.0]);
    assert_eq!(kernel_weights(BlurKernel::Gaussian, 0), vec![1.0]);

    let img = test_image();
    let dir = temp_dir("radius_zero_is_a_no_op");
    let source_path = dir.join("source.png");
    img.save(&source_path).unwrap();

    let weights = kernel_weights(BlurKernel::Gaussian, 0);
    let output_path = dir.join("out.png");
    let source = open_streamable_blur_source(&source_path, TextureFormat::Png)
        .unwrap()
        .unwrap();
    stream_blur(
        source,
        &weights,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    assert_eq!(actual, img);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn sixteen_bit_png_falls_back_to_buffered_blur() {
    let mut img16 = image::ImageBuffer::<image::Rgba<u16>, Vec<u16>>::new(6, 5);
    for (x, y, px) in img16.enumerate_pixels_mut() {
        *px = image::Rgba([
            (x * 4000) as u16,
            (y * 8000) as u16,
            ((x + y) * 2000) as u16,
            u16::MAX,
        ]);
    }

    let dir = temp_dir("sixteen_bit_png_falls_back_to_buffered_blur");
    let source_path = dir.join("source16.png");
    img16.save(&source_path).unwrap();

    assert!(
        open_streamable_blur_source(&source_path, TextureFormat::Png)
            .unwrap()
            .is_none()
    );

    let output_path = dir.join("out.png");
    let weights = kernel_weights(BlurKernel::Box, 1);
    buffered_blur(
        &source_path,
        &output_path,
        TextureFormat::Png,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let out = image::open(&output_path).unwrap();
    assert_eq!((out.width(), out.height()), (6, 5));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn blur_row_horizontal_masked_excludes_invalid_taps() {
    // Same row/weights as `blur_row_horizontal_clamps_at_edges`, but index 1
    // is masked out (invalid), so it should never contribute — and the
    // taps that *do* land on it should just shrink the denominator instead
    // of being treated as zero.
    let row = [10u8, 20, 30, 40];
    let valid = [true, false, true, true];
    let weights = [0.25f32, 0.5, 0.25];

    let (numerator, denominator) = blur_row_horizontal_masked(&row, &valid, 1, &weights);

    // x=0: dx=-1->sx=0(valid,10,w.25); dx=0->sx=0(valid,10,w.5); dx=+1->sx=1(invalid, skipped)
    //   numerator = .25*10 + .5*10 = 7.5, denominator = .25+.5 = 0.75
    // x=1: dx=-1->sx=0(valid,10,w.25); dx=0->sx=1(invalid, skipped); dx=+1->sx=2(valid,30,w.25)
    //   numerator = .25*10 + .25*30 = 10.0, denominator = .25+.25 = 0.5
    // x=2: dx=-1->sx=1(invalid, skipped); dx=0->sx=2(valid,30,w.5); dx=+1->sx=3(valid,40,w.25)
    //   numerator = .5*30 + .25*40 = 25.0, denominator = .5+.25 = 0.75
    // x=3: dx=-1->sx=2(valid,30,w.25); dx=0->sx=3(valid,40,w.5); dx=+1->sx=3 clamped(valid,40,w.25)
    //   numerator = .25*30 + .5*40 + .25*40 = 37.5, denominator = .25+.5+.25 = 1.0
    assert_eq!(numerator, vec![7.5, 10.0, 25.0, 37.5]);
    assert_eq!(denominator, vec![0.75, 0.5, 0.75, 1.0]);
}

fn checkered_mask(width: u32, height: u32) -> GrayImage {
    let mut mask = GrayImage::new(width, height);
    for (x, y, px) in mask.enumerate_pixels_mut() {
        *px = Luma([if (x + y) % 3 == 0 { 0 } else { 255 }]);
    }
    mask
}

#[test]
fn streaming_png_masked_box_blur_matches_buffered() {
    let img = test_image();
    let mask = checkered_mask(img.width(), img.height());

    let dir = temp_dir("streaming_png_masked_box_blur_matches_buffered");
    let source_path = dir.join("source.png");
    let mask_path = dir.join("mask.png");
    img.save(&source_path).unwrap();
    mask.save(&mask_path).unwrap();

    let weights = kernel_weights(BlurKernel::Box, 2);
    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: true, // --white-mask: white pixels excluded
    };

    let streamed_path = dir.join("streamed.png");
    let (image_source, mask_source) =
        open_streamable_masked_blur_sources(&source_path, mask_spec, TextureFormat::Png)
            .unwrap()
            .unwrap();
    stream_masked_blur(
        image_source,
        mask_source,
        mask_spec.excludes_white,
        &weights,
        &streamed_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let buffered_path = dir.join("buffered.png");
    buffered_masked_blur(
        &source_path,
        mask_spec,
        &buffered_path,
        TextureFormat::Png,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_masked_gaussian_blur_matches_buffered() {
    let img = test_image();
    let mask = checkered_mask(img.width(), img.height());

    let dir = temp_dir("streaming_tiff_masked_gaussian_blur_matches_buffered");
    let source_path = dir.join("source.tiff");
    let mask_path = dir.join("mask.png"); // mixed formats: TIFF image, PNG mask
    img.save(&source_path).unwrap();
    mask.save(&mask_path).unwrap();

    let weights = kernel_weights(BlurKernel::Gaussian, 3);
    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false, // --black-mask: black pixels excluded
    };

    let streamed_path = dir.join("streamed.tiff");
    let (image_source, mask_source) =
        open_streamable_masked_blur_sources(&source_path, mask_spec, TextureFormat::Tiff)
            .unwrap()
            .unwrap();
    stream_masked_blur(
        image_source,
        mask_source,
        mask_spec.excludes_white,
        &weights,
        &streamed_path,
        TextureFormat::Tiff,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let buffered_path = dir.join("buffered.tiff");
    buffered_masked_blur(
        &source_path,
        mask_spec,
        &buffered_path,
        TextureFormat::Tiff,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_output_equals_original_value() {
    // Uniform background with one wildly different outlier pixel at (4, 3),
    // masked out. This gives exact, easy-to-reason-about expectations:
    // - the outlier's own output must equal its original (unblurred) value
    // - a neighbor's output must equal the background exactly, since with
    //   the outlier excluded every remaining pixel in its window is the
    //   same uniform color (a weighted average of identical values is
    //   exactly that value, regardless of the weights)
    // - the same neighbor, blurred *without* masking, must differ — proving
    //   the outlier really would bleed in if it weren't excluded, i.e. that
    //   masking is actually doing something here.
    const BACKGROUND: Rgba<u8> = Rgba([100, 100, 100, 255]);
    const OUTLIER: Rgba<u8> = Rgba([255, 0, 0, 255]);

    let mut img = RgbaImage::new(9, 7);
    for px in img.pixels_mut() {
        *px = BACKGROUND;
    }
    img.put_pixel(4, 3, OUTLIER);

    let mut mask = GrayImage::new(9, 7);
    for px in mask.pixels_mut() {
        *px = Luma([255]);
    }
    mask.put_pixel(4, 3, Luma([0]));

    let dir = temp_dir("masked_pixel_output_equals_original_value");
    let source_path = dir.join("source.png");
    let mask_path = dir.join("mask.png");
    img.save(&source_path).unwrap();
    mask.save(&mask_path).unwrap();

    let weights = kernel_weights(BlurKernel::Gaussian, 2);
    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false, // --black-mask: black (the outlier pixel's mask) excluded
    };

    let streamed_path = dir.join("streamed.png");
    let (image_source, mask_source) =
        open_streamable_masked_blur_sources(&source_path, mask_spec, TextureFormat::Png)
            .unwrap()
            .unwrap();
    stream_masked_blur(
        image_source,
        mask_source,
        mask_spec.excludes_white,
        &weights,
        &streamed_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let buffered_path = dir.join("buffered.png");
    buffered_masked_blur(
        &source_path,
        mask_spec,
        &buffered_path,
        TextureFormat::Png,
        false,
        &weights,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    assert_eq!(*streamed.get_pixel(4, 3), OUTLIER);
    assert_eq!(*streamed.get_pixel(3, 3), BACKGROUND);

    // Without masking, the outlier bleeds into its neighbor's blur.
    let unmasked_path = dir.join("unmasked.png");
    stream_blur(
        open_streamable_blur_source(&source_path, TextureFormat::Png)
            .unwrap()
            .unwrap(),
        &weights,
        &unmasked_path,
        TextureFormat::Png,
        DisplayFormat::Json,
        "blur",
    )
    .unwrap();
    let unmasked = image::open(&unmasked_path).unwrap().to_rgba8();
    assert_ne!(*unmasked.get_pixel(3, 3), BACKGROUND);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mismatched_mask_dimensions_are_rejected() {
    let dir = temp_dir("mismatched_mask_dimensions_are_rejected");
    let source_path = dir.join("source.png");
    let mask_path = dir.join("mask.png");
    RgbaImage::new(6, 5).save(&source_path).unwrap();
    GrayImage::new(4, 4).save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: true,
    };
    assert!(matches!(
        open_streamable_masked_blur_sources(&source_path, mask_spec, TextureFormat::Png),
        Err(ProcessError::InvalidInput(_))
    ));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn gray_alpha_png_with_tiff_output_falls_back_to_buffered_blur() {
    let mut gray_alpha = GrayAlphaImage::new(5, 5);
    for (x, y, px) in gray_alpha.enumerate_pixels_mut() {
        *px = LumaA([(x * 10 + y) as u8, 200]);
    }

    let dir = temp_dir("gray_alpha_png_with_tiff_output_falls_back_to_buffered_blur");
    let source_path = dir.join("gray_alpha.png");
    gray_alpha.save(&source_path).unwrap();

    assert!(
        open_streamable_blur_source(&source_path, TextureFormat::Png)
            .unwrap()
            .is_some()
    );
    assert!(
        open_streamable_blur_source(&source_path, TextureFormat::Tiff)
            .unwrap()
            .is_none()
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
