use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{
    data::TileTree,
    render::{TerrainViewUniform, TileTreeUniform},
    view::TerrainViewComponents,
};

pub fn update_terrain_view_buffer(
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    for tile_tree in tile_trees.values_mut() {
        if !tile_tree.dirty {
            continue;
        }

        {
            let mut terrain_view_buffer = buffers.get_mut(&tile_tree.terrain_view_buffer).unwrap();
            terrain_view_buffer.set_data(TerrainViewUniform::from(&*tile_tree));
        }

        let mut tile_tree_buffer = buffers.get_mut(&tile_tree.tile_tree_buffer).unwrap();
        tile_tree_buffer.set_data(TileTreeUniform {
            entries: tile_tree.data.clone().into_iter().collect(),
        });

        tile_tree.dirty = false;
    }
}
