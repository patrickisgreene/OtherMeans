use bevy::{camera::primitives::Frustum, prelude::*};
use big_space::prelude::*;

use crate::{data::TileTree, view::TerrainViewComponents};

/// Traverses all tile_trees and updates the tile states,
/// while selecting newly requested and released tiles.
pub fn compute_requests(
    camera: Query<&Camera>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    grids: Grids,
    views: Query<(Ref<Transform>, &CellCoord)>,
) {
    for (&(_, view), tile_tree) in tile_trees.iter_mut() {
        let (transform, cell) = views.get(view).unwrap();

        if !transform.is_changed() {
            continue;
        }

        let camera = camera.get(view).unwrap();
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
        tile_tree.update();
        tile_tree.dirty = true;
    }
}
