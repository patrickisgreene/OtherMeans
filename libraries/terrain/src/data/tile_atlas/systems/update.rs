use bevy::prelude::*;

use crate::{
    data::{TileAtlas, TileTree},
    view::TerrainViewComponents,
};

/// Updates the tile atlas according to all corresponding tile_trees.
pub fn update(
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    mut tile_atlases: Query<&mut TileAtlas>,
) {
    for (&(terrain, _view), tile_tree) in tile_trees.iter_mut() {
        let mut tile_atlas = tile_atlases.get_mut(terrain).unwrap();

        for tile_coordinate in tile_tree.released_tiles.drain(..) {
            tile_atlas.release_tile(tile_coordinate);
        }

        for tile_coordinate in tile_tree.requested_tiles.drain(..) {
            tile_atlas.request_tile(tile_coordinate);
        }
    }
}
