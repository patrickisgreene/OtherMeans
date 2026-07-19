use bevy::{prelude::*, render::gpu_readback::ReadbackComplete};

use crate::{
    data::{TerrainViewKey, TileTree},
    view::TerrainViewComponents,
};

pub fn approximate_height_readback(
    trigger: On<ReadbackComplete>,
    terrain_view: Query<&TerrainViewKey>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
) {
    let TerrainViewKey(terrain_view) = terrain_view.get(trigger.entity).unwrap();
    let tile_tree = tile_trees.get_mut(terrain_view).unwrap();
    tile_tree.approximate_height = trigger.event().to_shader_type();
}
