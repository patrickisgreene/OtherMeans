use crate::*;
use image::{GrayImage, Luma, Rgba, RgbaImage};
use texture_processor::{
    DisplayFormat,
    utils::{self, MaskSpec, contrast_transform},
};

#[test]
fn streaming_png_contrast_matches_buffered_contrast() {
    let img = test_image();
    let dir = temp_dir("streaming_png_contrast_matches_buffered_contrast");
    let input_path = dir.join("in.png");
    img.save(&input_path).unwrap();

    let streamed_path = dir.join("streamed.png");
    assert!(
        utils::stream_png_pointwise(
            &input_path,
            &streamed_path,
            false,
            contrast_transform(35.0),
            DisplayFormat::Json,
            "contrast",
        )
        .unwrap()
    );

    let mut expected = image::DynamicImage::ImageRgba8(img);
    image::imageops::colorops::contrast_in_place(&mut expected, 35.0);

    let actual = image::open(&streamed_path).unwrap();
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_passes_through_contrast() {
    let mut img = RgbaImage::new(5, 5);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = Rgba([(x * 40) as u8, (y * 40) as u8, 100, 180]);
    }
    let mut mask = GrayImage::new(5, 5);
    for px in mask.pixels_mut() {
        *px = Luma([255]); // all white...
    }
    mask.put_pixel(2, 2, Luma([0])); // ...except one black (masked) pixel

    let dir = temp_dir("masked_pixel_passes_through_contrast");
    let input_path = dir.join("in.png");
    let mask_path = dir.join("mask.png");
    img.save(&input_path).unwrap();
    mask.save(&mask_path).unwrap();

    let mask_spec = MaskSpec {
        path: &mask_path,
        excludes_white: false, // --black-mask: black excluded
    };
    let contrast_byte = contrast_transform(35.0);

    let output_path = dir.join("out.png");
    assert!(
        utils::stream_png_pointwise_masked(
            &input_path,
            &output_path,
            false,
            contrast_byte,
            mask_spec,
            DisplayFormat::Json,
            "contrast",
        )
        .unwrap()
    );

    let actual = image::open(&output_path).unwrap().to_rgba8();

    // Masked pixel: untouched.
    assert_eq!(*actual.get_pixel(2, 2), *img.get_pixel(2, 2));

    // Unmasked pixel: contrast applied to every channel, alpha included
    // (contrast is not alpha-exempt, unlike invert/brighten).
    let source = img.get_pixel(0, 0).0;
    let expected = Rgba([
        contrast_byte(source[0]),
        contrast_byte(source[1]),
        contrast_byte(source[2]),
        contrast_byte(source[3]),
    ]);
    assert_eq!(*actual.get_pixel(0, 0), expected);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_contrast_matches_buffered_contrast() {
    let img = test_image();
    let dir = temp_dir("streaming_tiff_contrast_matches_buffered_contrast");
    let input_path = dir.join("in.tiff");
    img.save(&input_path).unwrap();

    let streamed_path = dir.join("streamed.tiff");
    assert!(
        utils::stream_tiff_pointwise(
            &input_path,
            &streamed_path,
            false,
            contrast_transform(-20.0),
            DisplayFormat::Json,
            "contrast",
        )
        .unwrap()
    );

    let mut expected = image::DynamicImage::ImageRgba8(img);
    image::imageops::colorops::contrast_in_place(&mut expected, -20.0);

    let actual = image::open(&streamed_path).unwrap();
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());

    std::fs::remove_dir_all(&dir).unwrap();
}
