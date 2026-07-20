use bevy::{
    prelude::*,
    render::{Extract, renderer::RenderDevice, texture::FallbackImage},
};

use crate::{
    config::TerrainComponents,
    data::{TileAtlas, tile_atlas::gpu::GpuTileAtlas},
    render::GpuTerrain,
};

pub fn initialize(
    device: Res<RenderDevice>,
    fallback_image: Res<FallbackImage>,
    mut gpu_terrains: ResMut<TerrainComponents<GpuTerrain>>,
    gpu_tile_atlases: Res<TerrainComponents<GpuTileAtlas>>,
    tile_atlases: Extract<Query<(Entity, &TileAtlas), Added<TileAtlas>>>,
) {
    for (terrain, tile_atlas) in &tile_atlases {
        let gpu_tile_atlas = &gpu_tile_atlases[&terrain];

        gpu_terrains.insert(
            terrain,
            GpuTerrain::new(&device, &fallback_image, tile_atlas, gpu_tile_atlas),
        );
    }
}
