use crate::*;
use image::{GrayImage, Luma, Rgba, RgbaImage};
use texture_processor::{
    DisplayFormat,
    utils::{self, MaskSpec},
};

#[test]
fn streaming_png_brightness_matches_buffered_brighten() {
    let img = test_image();
    let dir = temp_dir("streaming_png_brightness_matches_buffered_brighten");
    let input_path = dir.join("in.png");
    img.save(&input_path).unwrap();

    let streamed_path = dir.join("streamed.png");
    assert!(
        utils::stream_png_pointwise(
            &input_path,
            &streamed_path,
            true,
            |b| (i32::from(b) + 40).clamp(0, 255) as u8,
            DisplayFormat::Json,
            "brighten",
        )
        .unwrap()
    );

    let mut expected = image::DynamicImage::ImageRgba8(img);
    image::imageops::colorops::brighten_in_place(&mut expected, 40);

    let actual = image::open(&streamed_path).unwrap();
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn masked_pixel_passes_through_brighten() {
    let mut img = RgbaImage::new(5, 5);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = Rgba([(x * 40) as u8, (y * 40) as u8, 100, 255]);
    }
    let mut mask = GrayImage::new(5, 5);
    for px in mask.pixels_mut() {
        *px = Luma([255]); // all white...
    }
    mask.put_pixel(2, 2, Luma([0])); // ...except one black (masked) pixel

    let dir = temp_dir("masked_pixel_passes_through_brighten");
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
            |b| (i32::from(b) + 40).clamp(0, 255) as u8,
            mask_spec,
            DisplayFormat::Json,
            "brighten",
        )
        .unwrap()
    );

    let actual = image::open(&output_path).unwrap().to_rgba8();

    // Masked pixel: untouched.
    assert_eq!(*actual.get_pixel(2, 2), *img.get_pixel(2, 2));

    // Unmasked pixel: brightened, alpha exempt.
    let source = img.get_pixel(0, 0).0;
    let expected = Rgba([
        (i32::from(source[0]) + 40).clamp(0, 255) as u8,
        (i32::from(source[1]) + 40).clamp(0, 255) as u8,
        (i32::from(source[2]) + 40).clamp(0, 255) as u8,
        source[3],
    ]);
    assert_eq!(*actual.get_pixel(0, 0), expected);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_brightness_matches_buffered_brighten() {
    let img = test_image();
    let dir = temp_dir("streaming_tiff_brightness_matches_buffered_brighten");
    let input_path = dir.join("in.tiff");
    img.save(&input_path).unwrap();

    let streamed_path = dir.join("streamed.tiff");
    assert!(
        utils::stream_tiff_pointwise(
            &input_path,
            &streamed_path,
            true,
            |b| (i32::from(b) - 60).clamp(0, 255) as u8,
            DisplayFormat::Json,
            "brighten",
        )
        .unwrap()
    );

    let mut expected = image::DynamicImage::ImageRgba8(img);
    image::imageops::colorops::brighten_in_place(&mut expected, -60);

    let actual = image::open(&streamed_path).unwrap();
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());

    std::fs::remove_dir_all(&dir).unwrap();
}
