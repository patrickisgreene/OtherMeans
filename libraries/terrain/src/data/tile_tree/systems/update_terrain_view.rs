use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{
    data::TileTree,
    render::{TerrainViewUniform, TileTreeUniform},
};

pub fn update_terrain_view_buffer(
    mut tile_trees: Query<&mut TileTree>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    for mut tile_tree in tile_trees.iter_mut() {
        if tile_tree.dirty {
            let mut terrain_view_buffer = buffers.get_mut(&tile_tree.terrain_view_buffer).unwrap();
            terrain_view_buffer.set_data(TerrainViewUniform::from(&*tile_tree));
        }

        if tile_tree.tiles_dirty {
            let mut tile_tree_buffer = buffers.get_mut(&tile_tree.tile_tree_buffer).unwrap();
            tile_tree_buffer.set_data(TileTreeUniform {
                entries: tile_tree.data.iter().copied().collect(),
            });
        }

        tile_tree.dirty = false;
        tile_tree.tiles_dirty = false;
    }
}
