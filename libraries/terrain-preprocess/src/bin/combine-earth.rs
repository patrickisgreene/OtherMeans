//! Merges the already-generated `land` and `water` tile atlases into a single `earth` attachment,
//! per-pixel-selected by the `height` attachment's own ocean sentinel (`height == 0` -> ocean,
//! matching `compute_ocean_blend` in `assets/shaders/earth/fragment.wgsl`). Land and water are
//! never sampled simultaneously at render time - the shader always knows which side of the
//! coastline it's on from height alone - so combining them ahead of time drops one whole
//! attachment/texture bind at runtime instead of blending two textures every frame.
//!
//! Must run after the `land`/`water`/`height` `terrain-preprocess` invocations have all produced
//! tiles at the same texture size (see `resources/earth/preprocess.sh`) - this only combines
//! pre-existing tile files, it doesn't reproject/downsample/fill anything itself.

use clap::Parser;
use gdal::{
    Dataset, DriverManager,
    raster::{Buffer, ColorInterpretation, RasterCreationOptions},
};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use terrain::prelude::*;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(long = "terrain-path", required = true)]
    terrain_path: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let config_path = args.terrain_path.join("terrain.ron");
    let mut config = TerrainConfig::load_file(&config_path)
        .unwrap_or_else(|e| panic!("failed to load {}: {e}", config_path.display()));

    let land_label = AttachmentLabel::Custom("land".into());
    let water_label = AttachmentLabel::Custom("water".into());
    let earth_label = AttachmentLabel::Custom("earth".into());

    let land_config = config.attachments[&land_label].clone();
    let water_config = config.attachments[&water_label].clone();
    let height_config = config.attachments[&AttachmentLabel::Height].clone();

    assert_eq!(
        land_config.texture_size, water_config.texture_size,
        "land and water must share a texture size to combine per-pixel - see the resolution note in preprocess.sh"
    );
    assert_eq!(
        land_config.texture_size, height_config.texture_size,
        "land/water and height must share a texture size to combine per-pixel"
    );
    let texture_size = land_config.texture_size as usize;

    let land_dir = args.terrain_path.join(String::from(&land_label));
    let water_dir = args.terrain_path.join(String::from(&water_label));
    let height_dir = args.terrain_path.join(String::from(&AttachmentLabel::Height));
    let earth_dir = args.terrain_path.join(String::from(&earth_label));

    config.tiles.par_iter().for_each(|&tile| {
        combine_tile(tile, &land_dir, &water_dir, &height_dir, &earth_dir, texture_size);
    });

    // Reuse water's attachment config (identical shape to land/height already) for the new
    // combined attachment, then drop the two it replaces.
    config.add_attachment(earth_label, water_config);
    config.attachments.remove(&land_label);
    config.attachments.remove(&water_label);
    config
        .save_file(&config_path)
        .unwrap_or_else(|e| panic!("failed to save {}: {e}", config_path.display()));

    std::fs::remove_dir_all(&land_dir).ok();
    std::fs::remove_dir_all(&water_dir).ok();

    println!(
        "Combined {} tiles into the earth attachment; removed land/water tile directories.",
        config.tiles.len()
    );
}

fn combine_tile(
    tile: TileCoordinate,
    land_dir: &Path,
    water_dir: &Path,
    height_dir: &Path,
    earth_dir: &Path,
    texture_size: usize,
) {
    let land = Dataset::open(tile.path(land_dir)).expect("failed to open land tile");
    let water = Dataset::open(tile.path(water_dir)).expect("failed to open water tile");
    let height = Dataset::open(tile.path(height_dir)).expect("failed to open height tile");

    let read_rgb = |dataset: &Dataset| -> [Buffer<u8>; 3] {
        std::array::from_fn(|i| {
            dataset
                .rasterband(i + 1)
                .unwrap()
                .read_band_as::<u8>()
                .unwrap()
        })
    };
    let land_bands = read_rgb(&land);
    let water_bands = read_rgb(&water);
    let height_data = height
        .rasterband(1)
        .unwrap()
        .read_band_as::<u16>()
        .unwrap();

    let pixels = texture_size * texture_size;
    let mut out_bands: [Vec<u8>; 3] = [
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
    ];
    for pixel in 0..pixels {
        let is_ocean = height_data.data()[pixel] == 0;
        for (channel, out) in out_bands.iter_mut().enumerate() {
            let source = if is_ocean { &water_bands } else { &land_bands };
            out.push(source[channel].data()[pixel]);
        }
    }

    let out_path = tile.path(earth_dir);
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();

    let driver = DriverManager::get_driver_by_name("GTiff").unwrap();
    let options = RasterCreationOptions::from_iter([
        "TILED=YES",
        "BLOCKXSIZE=512",
        "BLOCKYSIZE=512",
        "INTERLEAVE=PIXEL",
    ]);
    let mut dst = driver
        .create_with_band_type_with_options::<u8, _>(
            &out_path,
            texture_size,
            texture_size,
            3,
            &options,
        )
        .unwrap();

    if let Ok(geo_transform) = land.geo_transform() {
        dst.set_geo_transform(&geo_transform).ok();
    }

    for (i, band_data) in out_bands.into_iter().enumerate() {
        let mut buffer = Buffer::new((texture_size, texture_size), band_data);
        let mut dst_band = dst.rasterband(i + 1).unwrap();
        dst_band
            .write((0, 0), (texture_size, texture_size), &mut buffer)
            .unwrap();
        let color_interpretation = match i {
            0 => ColorInterpretation::RedBand,
            1 => ColorInterpretation::GreenBand,
            _ => ColorInterpretation::BlueBand,
        };
        dst_band.set_color_interpretation(color_interpretation).ok();
    }
}
