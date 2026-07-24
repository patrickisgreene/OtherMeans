use crate::data::{TileAtlas, TileTree};
use bevy::prelude::*;
use std::iter;

/// Adjusts all tile_trees to their corresponding tile atlas
/// by updating the entries with the best available tiles.
pub fn adjust_to_tile_atlas(mut terrains: Query<(&mut TileTree, &mut TileAtlas)>) {
    for (mut tile_tree, mut tile_atlas) in terrains.iter_mut() {
        if !tile_tree.tiles_dirty && !tile_atlas.changed {
            continue;
        }

        let tile_tree = &mut *tile_tree;

        for (tile, entry) in iter::zip(&tile_tree.tiles, &mut tile_tree.data) {
            *entry = tile_atlas.get_best_tile(tile.coordinate);
        }

        tile_tree.tiles_dirty = true;
        tile_atlas.changed = false;
    }
}
