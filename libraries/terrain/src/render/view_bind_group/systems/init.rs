use bevy::{
    prelude::*,
    render::{Extract, renderer::RenderDevice},
};

use crate::{data::TileTree, render::GpuTerrainView, view::TerrainViewComponents};

pub fn initialize(
    device: Res<RenderDevice>,
    mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
    tile_trees: Extract<Res<TerrainViewComponents<TileTree>>>,
) {
    for (&(terrain, view), tile_tree) in tile_trees.iter() {
        if gpu_terrain_views.contains_key(&(terrain, view)) {
            continue;
        }

        gpu_terrain_views.insert((terrain, view), GpuTerrainView::new(&device, tile_tree));
    }
}
