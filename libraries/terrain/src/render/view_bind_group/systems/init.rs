use bevy::{
    prelude::*,
    render::{Extract, renderer::RenderDevice, sync_world::RenderEntity},
};

use crate::{data::TileTree, render::GpuTerrainView};

pub fn initialize(
    mut commands: Commands,
    device: Res<RenderDevice>,
    tile_trees: Extract<Query<(RenderEntity, &TileTree), Added<TileTree>>>,
) {
    for (render_entity, tile_tree) in &tile_trees {
        commands
            .entity(render_entity)
            .insert(GpuTerrainView::new(&device, tile_tree));
    }
}
