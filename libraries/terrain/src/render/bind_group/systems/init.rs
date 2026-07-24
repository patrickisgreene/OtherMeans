use bevy::{
    prelude::*,
    render::{Extract, renderer::RenderDevice, sync_world::RenderEntity, texture::FallbackImage},
};

use crate::{
    data::{TileAtlas, tile_atlas::gpu::GpuTileAtlas},
    render::GpuTerrain,
};

pub fn initialize(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fallback_image: Res<FallbackImage>,
    gpu_tile_atlases: Query<&GpuTileAtlas>,
    tile_atlases: Extract<Query<(RenderEntity, &TileAtlas), Added<TileAtlas>>>,
) {
    for (render_entity, tile_atlas) in &tile_atlases {
        // Same race as `GpuTileAtlas::extract`: on the first frame `TileAtlas` was added,
        // `GpuTileAtlas::initialize`'s deferred insert may not be visible yet this pass.
        let Ok(gpu_tile_atlas) = gpu_tile_atlases.get(render_entity) else {
            continue;
        };

        commands.entity(render_entity).insert(GpuTerrain::new(
            &device,
            &fallback_image,
            tile_atlas,
            gpu_tile_atlas,
        ));
    }
}
