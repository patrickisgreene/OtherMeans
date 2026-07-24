use bevy::prelude::*;

use crate::data::{TileAtlas, TileTree};

/// Updates the tile atlas according to all corresponding tile_trees.
pub fn update(mut terrains: Query<(&mut TileAtlas, &mut TileTree)>) {
    for (mut tile_atlas, mut tile_tree) in terrains.iter_mut() {
        for tile_coordinate in tile_tree.released_tiles.drain(..) {
            tile_atlas.release_tile(tile_coordinate);
        }

        for tile_coordinate in tile_tree.requested_tiles.drain(..) {
            tile_atlas.request_tile(tile_coordinate);
        }
    }
}
