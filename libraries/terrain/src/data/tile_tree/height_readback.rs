use bevy::{prelude::*, render::gpu_readback::ReadbackComplete};

use crate::data::TileTree;

pub fn approximate_height_readback(
    trigger: On<ReadbackComplete>,
    mut tile_trees: Query<&mut TileTree>,
) {
    let mut tile_tree = tile_trees.single_mut().unwrap();
    tile_tree.approximate_height = trigger.event().to_shader_type();
}
