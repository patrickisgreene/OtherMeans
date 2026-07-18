use std::path::Path;

use crate::*;
use image::{GrayImage, Luma};
use texture_processor::{DisplayFormat, ProcessError, TextureFormat, utils::*};

#[test]
fn streaming_png_concat_reproduces_source_when_all_roles_match() {
    let img = test_image();
    let dir = temp_dir("streaming_png_concat_reproduces_source_when_all_roles_match");
    let source_path = dir.join("source.png");
    img.save(&source_path).unwrap();

    let roles: Vec<(Role, &Path)> = vec![
        (Role::Red, source_path.as_path()),
        (Role::Green, source_path.as_path()),
        (Role::Blue, source_path.as_path()),
        (Role::Alpha, source_path.as_path()),
    ];
    let sources = open_all_row_sources(&roles).unwrap().unwrap();

    let output_path = dir.join("out.png");
    stream_concat(
        sources,
        4,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    assert_eq!(actual, img);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_tiff_concat_reproduces_source_when_all_roles_match() {
    let img = test_image();
    let dir = temp_dir("streaming_tiff_concat_reproduces_source_when_all_roles_match");
    let source_path = dir.join("source.tiff");
    img.save(&source_path).unwrap();

    let roles: Vec<(Role, &Path)> = vec![
        (Role::Red, source_path.as_path()),
        (Role::Green, source_path.as_path()),
        (Role::Blue, source_path.as_path()),
        (Role::Alpha, source_path.as_path()),
    ];
    let sources = open_all_row_sources(&roles).unwrap().unwrap();

    let output_path = dir.join("out.tiff");
    stream_concat(
        sources,
        4,
        &output_path,
        TextureFormat::Tiff,
        DisplayFormat::Json,
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgba8();
    assert_eq!(actual, img);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn streaming_grayscale_sources_fill_rgb_with_their_only_channel() {
    let mut gray = GrayImage::new(9, 4);
    for (x, y, px) in gray.enumerate_pixels_mut() {
        *px = Luma([(x * 11 + y * 3) as u8]);
    }

    let dir = temp_dir("streaming_grayscale_sources_fill_rgb_with_their_only_channel");
    let gray_path = dir.join("gray.png");
    gray.save(&gray_path).unwrap();

    let roles: Vec<(Role, &Path)> = vec![
        (Role::Red, gray_path.as_path()),
        (Role::Green, gray_path.as_path()),
        (Role::Blue, gray_path.as_path()),
    ];
    let sources = open_all_row_sources(&roles).unwrap().unwrap();

    let output_path = dir.join("out.png");
    stream_concat(
        sources,
        3,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
    )
    .unwrap();

    let actual = image::open(&output_path).unwrap().to_rgb8();
    for (x, y, pixel) in actual.enumerate_pixels() {
        let luma = gray.get_pixel(x, y).0[0];
        assert_eq!(pixel.0, [luma, luma, luma]);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn buffered_fallback_reproduces_source_when_all_roles_match() {
    // 16-bit depth forces the PNG/TIFF fast paths to bail, exercising
    // `buffered_concat` instead of `stream_concat`.
    let mut img16 = image::ImageBuffer::<image::Rgba<u16>, Vec<u16>>::new(6, 5);
    for (x, y, px) in img16.enumerate_pixels_mut() {
        *px = image::Rgba([
            (x * 4000) as u16,
            (y * 8000) as u16,
            ((x + y) * 2000) as u16,
            u16::MAX,
        ]);
    }

    let dir = temp_dir("buffered_fallback_reproduces_source_when_all_roles_match");
    let source_path = dir.join("source16.png");
    img16.save(&source_path).unwrap();

    let roles: Vec<(Role, &Path)> = vec![
        (Role::Red, source_path.as_path()),
        (Role::Green, source_path.as_path()),
        (Role::Blue, source_path.as_path()),
        (Role::Alpha, source_path.as_path()),
    ];
    assert!(open_all_row_sources(&roles).unwrap().is_none());

    let output_path = dir.join("out.png");
    buffered_concat(
        &roles,
        4,
        &output_path,
        TextureFormat::Png,
        false,
        DisplayFormat::Json,
    )
    .unwrap();

    let expected = image::open(&source_path).unwrap().to_rgba8();
    let actual = image::open(&output_path).unwrap().to_rgba8();
    assert_eq!(actual, expected);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mismatched_dimensions_are_rejected() {
    let dir = temp_dir("mismatched_dimensions_are_rejected");
    let small = GrayImage::new(4, 4);
    let big = GrayImage::new(8, 8);
    let small_path = dir.join("small.png");
    let big_path = dir.join("big.png");
    small.save(&small_path).unwrap();
    big.save(&big_path).unwrap();

    let roles: Vec<(Role, &Path)> = vec![
        (Role::Red, small_path.as_path()),
        (Role::Green, big_path.as_path()),
        (Role::Blue, small_path.as_path()),
    ];

    assert!(matches!(
        open_all_row_sources(&roles),
        Err(ProcessError::InvalidInput(_))
    ));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn alpha_role_without_alpha_source_is_rejected() {
    let dir = temp_dir("alpha_role_without_alpha_source_is_rejected");
    let rgb = image::RgbImage::new(4, 4);
    let rgb_path = dir.join("rgb.png");
    rgb.save(&rgb_path).unwrap();

    let roles: Vec<(Role, &Path)> = vec![
        (Role::Red, rgb_path.as_path()),
        (Role::Green, rgb_path.as_path()),
        (Role::Blue, rgb_path.as_path()),
        (Role::Alpha, rgb_path.as_path()),
    ];
    let sources = open_all_row_sources(&roles).unwrap().unwrap();

    let output_path = dir.join("out.png");
    let result = stream_concat(
        sources,
        4,
        &output_path,
        TextureFormat::Png,
        DisplayFormat::Json,
    );

    assert!(matches!(result, Err(ProcessError::InvalidInput(_))));

    std::fs::remove_dir_all(&dir).unwrap();
}
