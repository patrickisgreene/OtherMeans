use gdal::Dataset;
use gdal::spatial_ref::SpatialRef;
use texture_processor::TextureFormat;
use texture_processor::distance::write_raster;

use crate::temp_dir;

const WIDTH: usize = 4;
const HEIGHT: usize = 3;
const GEO_TRANSFORM: [f64; 6] = [10.0, 2.0, 0.0, 20.0, 0.0, -2.0];

fn wgs84() -> SpatialRef {
    SpatialRef::from_proj4("+proj=lonlat +ellps=WGS84 +datum=WGS84").unwrap()
}

fn test_data() -> Vec<u8> {
    (0..(WIDTH * HEIGHT) as u32).map(|i| (i * 17) as u8).collect()
}

#[test]
fn tiff_output_preserves_georeferencing_and_pixels() {
    let dir = temp_dir("tiff_output_preserves_georeferencing_and_pixels");
    let path = dir.join("out.tiff");
    let data = test_data();

    write_raster(
        &path,
        data.clone(),
        WIDTH,
        HEIGHT,
        TextureFormat::Tiff,
        GEO_TRANSFORM,
        &wgs84(),
    )
    .unwrap();

    let ds = Dataset::open(&path).unwrap();
    assert_eq!(ds.raster_size(), (WIDTH, HEIGHT));
    assert_eq!(ds.geo_transform().unwrap(), GEO_TRANSFORM);

    let band = ds.rasterband(1).unwrap();
    let buf = band
        .read_as::<u8>((0, 0), (WIDTH, HEIGHT), (WIDTH, HEIGHT), None)
        .unwrap();
    assert_eq!(buf.data(), data.as_slice());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn png_output_preserves_pixels() {
    let dir = temp_dir("png_output_preserves_pixels");
    let path = dir.join("out.png");
    let data = test_data();

    write_raster(
        &path,
        data.clone(),
        WIDTH,
        HEIGHT,
        TextureFormat::Png,
        GEO_TRANSFORM,
        &wgs84(),
    )
    .unwrap();

    let actual = image::open(&path).unwrap().to_luma8();
    assert_eq!(actual.dimensions(), (WIDTH as u32, HEIGHT as u32));
    assert_eq!(actual.into_raw(), data);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn tiff_and_png_outputs_agree_on_pixel_data() {
    let dir = temp_dir("tiff_and_png_outputs_agree_on_pixel_data");
    let tiff_path = dir.join("out.tiff");
    let png_path = dir.join("out.png");
    let data = test_data();

    write_raster(
        &tiff_path,
        data.clone(),
        WIDTH,
        HEIGHT,
        TextureFormat::Tiff,
        GEO_TRANSFORM,
        &wgs84(),
    )
    .unwrap();
    write_raster(
        &png_path,
        data.clone(),
        WIDTH,
        HEIGHT,
        TextureFormat::Png,
        GEO_TRANSFORM,
        &wgs84(),
    )
    .unwrap();

    let from_tiff = image::open(&tiff_path).unwrap().to_luma8();
    let from_png = image::open(&png_path).unwrap().to_luma8();
    assert_eq!(from_tiff, from_png);

    std::fs::remove_dir_all(&dir).unwrap();
}
