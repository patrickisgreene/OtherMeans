use bevy::prelude::*;

use crate::data::TileTree;

pub fn generate_surface_approximation(mut tile_trees: Query<&mut TileTree>) {
    for mut tile_tree in tile_trees.iter_mut() {
        if !tile_tree.dirty {
            continue;
        }

        tile_tree.surface_approximation = tile_tree.view_coordinates.map(|view_coordinate| {
            crate::math::SurfaceApproximation::compute(
                view_coordinate,
                tile_tree.view_local_position,
                tile_tree.view_world_position,
                tile_tree.shape,
            )
        });
    }
}
