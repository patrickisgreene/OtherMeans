use bevy::{camera::primitives::Frustum, prelude::*};
use big_space::prelude::*;

use crate::{data::TileTree, view::TerrainViewConfig};

/// Traverses all tile_trees and updates the tile states,
/// while selecting newly requested and released tiles.
pub fn compute_requests(
    mut tile_trees: Query<&mut TileTree>,
    grids: Grids,
    views: Query<(Entity, Ref<Transform>, &CellCoord, &Camera), With<TerrainViewConfig>>,
) {
    for mut tile_tree in tile_trees.iter_mut() {
        let (view, transform, cell, camera) = views.single().unwrap();

        if !transform.is_changed() && tile_tree.initialized {
            continue;
        }
        tile_tree.initialized = true;
        let grid = grids.parent_grid(view).unwrap();

        // Todo: transform should be global transform?

        let clip_from_view = camera.clip_from_view();
        let world_from_view = transform.to_matrix();
        let clip_from_world = clip_from_view * world_from_view.inverse();

        let half_spaces = Frustum(ViewFrustum::from_clip_from_world(&clip_from_world))
            .half_spaces
            .map(|space| space.normal_d());

        tile_tree.view_local_position = grid.grid_position_double(cell, &transform);
        tile_tree.view_world_position = transform.translation;
        tile_tree.half_spaces = half_spaces;
        tile_tree.dirty = true;
        if tile_tree.update() {
            tile_tree.tiles_dirty = true;
        }
    }
}
