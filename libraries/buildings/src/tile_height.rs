use std::path::PathBuf;

use bevy::{
    image::Image,
    math::DVec2,
    prelude::*,
};
use terrain::data::attachment::Attachment;
use terrain::prelude::AttachmentLabel;
use terrain::prelude::TileCoordinate;

/// Asset-relative path to `coordinate`'s own height tile - the exact R32F file the terrain
/// renderer itself loads to displace that tile's mesh (see
/// `libraries/terrain/src/data/tile_loader/default_loader.rs`'s `start_loading`, which builds
/// the same path from the same `Attachment`).
pub fn height_tile_path(coordinate: TileCoordinate, height_attachment: &Attachment) -> PathBuf {
    coordinate.path(&height_attachment.path().join(String::from(&AttachmentLabel::Height)))
}

/// Bilinearly samples a loaded height tile `image` (the terrain's own per-tile R32F attachment,
/// loaded via [`height_tile_path`]) at a tile-local UV (`0..1` over the tile's geographic
/// footprint, clamped). Replicates the GPU's border/scale remap
/// (`libraries/terrain/src/render/bind_group/attachment.rs`'s `AttachmentConfig::new`,
/// `libraries/terrain/src/shaders/attachments.wgsl`'s `compute_sample_uv`) so this always
/// agrees with what the terrain mesh itself displaced to at that point, instead of some
/// independently-resampled approximation.
///
/// Returns the raw normalized height value - same convention as the GPU's `sample_height`
/// before its `terrain.height_scale *` multiply, so callers keep multiplying by
/// `TileAtlas::height_scale` themselves exactly as before.
pub fn sample_height_tile(image: &Image, height_attachment: &Attachment, local_uv: DVec2) -> f32 {
    let texture_size = height_attachment.texture_size();
    let scale = height_attachment.center_size() as f64 / texture_size as f64;
    let offset = height_attachment.border_size() as f64 / texture_size as f64;

    let padded_uv = local_uv.clamp(DVec2::ZERO, DVec2::ONE) * scale + offset;
    // Half-texel offset so `pixel` lands on texel centers, matching standard bilinear/hardware
    // texture sampling conventions.
    let pixel = padded_uv * texture_size as f64 - 0.5;

    let data: &[f32] = bytemuck::cast_slice(
        image
            .data
            .as_ref()
            .expect("height tile image missing CPU-side data"),
    );

    let sample = |x: i64, y: i64| -> f32 {
        let x = x.clamp(0, texture_size as i64 - 1) as u32;
        let y = y.clamp(0, texture_size as i64 - 1) as u32;
        data[(y * texture_size + x) as usize]
    };

    let x0 = pixel.x.floor();
    let y0 = pixel.y.floor();
    let fx = (pixel.x - x0) as f32;
    let fy = (pixel.y - y0) as f32;
    let (x0, y0) = (x0 as i64, y0 as i64);

    let top = sample(x0, y0) + (sample(x0 + 1, y0) - sample(x0, y0)) * fx;
    let bottom = sample(x0, y0 + 1) + (sample(x0 + 1, y0 + 1) - sample(x0, y0 + 1)) * fx;
    top + (bottom - top) * fy
}

/// Tile-local UV (`0..1` over `target`'s own geographic footprint) of a point elsewhere given by
/// its face-level UV - the fractional part of `face_uv * tile_count`, matching how
/// `libraries/terrain/src/math/coordinate.rs`'s `ViewCoordinate::new` and
/// `roads::index::tile_xy` both relate a face UV to a specific tile.
pub fn tile_local_uv(face_uv: DVec2, target: TileCoordinate) -> DVec2 {
    let tile_count = 2f64.powi(target.lod as i32);
    face_uv * tile_count - target.xy.as_dvec2()
}
