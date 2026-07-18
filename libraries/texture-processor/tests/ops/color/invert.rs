use crate::*;
use image::{GrayImage, Luma, Rgba, RgbaImage};
use texture_processor::{
    DisplayFormat,
    utils::{self, MaskSpec, invert_byte},
};

#[test]
fn streaming_png_invert_matches_buffered_invert() {
    let img = test_image();
    let dir = temp_dir("streaming_png_invert_matches_buffered_invert");
    let input_path = dir.join("in.png");
    img.save(&input_path).unwrap();

    let streamed_path = dir.join("streamed.png");
    assert!(
        utils::stream_png_pointwise(
            &input_path,
            &streamed_path,
            true,
            invert_byte,
            DisplayFormat::Json,
            "invert",
        )
        .unwrap()
    );

    let mut expected = image::DynamicImage::ImageRgba8(img);
    image::imageops::invert(&mut expected);

    let actual = image::open(&streamed_path).unwrap();
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_invert_matches_buffered_invert() {
    let img = test_image();
    let dir = temp_dir("streaming_tiff_invert_matches_buffered_invert");
    let input_path = dir.join("in.tiff");
    img.save(&input_path).unwrap();

    let streamed_path = dir.join("streamed.tiff");
    assert!(
        utils::stream_tiff_pointwise(
            &input_path,
            &streamed_path,
            true,
            invert_byte,
            DisplayFormat::Json,
            "invert",
        )
        .unwrap()
    );

    let mut expected = image::DynamicImage::ImageRgba8(img);
    image::imageops::invert(&mut expected);

    let actual = image::open(&streamed_path).unwrap();
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());

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
fn streaming_png_masked_invert_matches_buffered() {
    let img = test_image();
    let mask = checkered_mask(img.width(), img.height());

    let dir = temp_dir("streaming_png_masked_invert_matches_buffered");
    let input_path = dir.join("in.png");
    let mask_path = dir.join("mask.png");
    img.save(&input_path).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: true,
    };

    let streamed_path = dir.join("streamed.png");
    assert!(
        utils::stream_png_pointwise_masked(
            &input_path,
            &streamed_path,
            true,
            invert_byte,
            mask_spec,
            DisplayFormat::Json,
            "invert",
        )
        .unwrap()
    );

    let buffered_path = dir.join("buffered.png");
    utils::buffered_fallback_masked(
        &input_path,
        &buffered_path,
        image::ImageFormat::Png,
        false,
        true,
        invert_byte,
        mask_spec,
        DisplayFormat::Json,
        "invert",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_masked_invert_matches_buffered() {
    let img = test_image();
    let mask = checkered_mask(img.width(), img.height());

    let dir = temp_dir("streaming_tiff_masked_invert_matches_buffered");
    let input_path = dir.join("in.tiff");
    let mask_path = dir.join("mask.png"); // mixed formats: TIFF image, PNG mask
    img.save(&input_path).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false,
    };

    let streamed_path = dir.join("streamed.tiff");
    assert!(
        utils::stream_tiff_pointwise_masked(
            &input_path,
            &streamed_path,
            true,
            invert_byte,
            mask_spec,
            DisplayFormat::Json,
            "invert",
        )
        .unwrap()
    );

    let buffered_path = dir.join("buffered.tiff");
    utils::buffered_fallback_masked(
        &input_path,
        &buffered_path,
        image::ImageFormat::Tiff,
        false,
        true,
        invert_byte,
        mask_spec,
        DisplayFormat::Json,
        "invert",
    )
    .unwrap();

    let streamed = image::open(&streamed_path).unwrap().to_rgba8();
    let buffered = image::open(&buffered_path).unwrap().to_rgba8();
    assert_eq!(streamed, buffered);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_passes_through_unmasked_pixel_inverts() {
    let mut img = RgbaImage::new(5, 5);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = Rgba([(x * 40) as u8, (y * 40) as u8, 128, 255]);
    }
    let mut mask = GrayImage::new(5, 5);
    for px in mask.pixels_mut() {
        *px = Luma([255]); // all white...
    }
    mask.put_pixel(2, 2, Luma([0])); // ...except one black (masked) pixel

    let dir = temp_dir("masked_pixel_passes_through_unmasked_pixel_inverts");
    let input_path = dir.join("in.png");
    let mask_path = dir.join("mask.png");
    img.save(&input_path).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false, // --black-mask: black excluded
    };

    let output_path = dir.join("out.png");
    assert!(
        utils::stream_png_pointwise_masked(
            &input_path,
            &output_path,
            true,
            invert_byte,
            mask_spec,
            DisplayFormat::Json,
            "invert",
        )
        .unwrap()
    );

    let actual = image::open(&output_path).unwrap().to_rgba8();

    // Masked pixel: untouched.
    assert_eq!(*actual.get_pixel(2, 2), *img.get_pixel(2, 2));

    // Unmasked pixel: inverted, alpha exempt (matches `invert`'s alpha_exempt=true).
    let source = img.get_pixel(0, 0).0;
    let expected = Rgba([255 - source[0], 255 - source[1], 255 - source[2], source[3]]);
    assert_eq!(*actual.get_pixel(0, 0), expected);

    std::fs::remove_dir_all(&dir).unwrap();
}
