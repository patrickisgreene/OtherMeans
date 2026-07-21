use crate::{
    data::{TileAtlas, TileTree},
    view::TerrainViewComponents,
};
use bevy::prelude::*;
use std::iter;

/// Adjusts all tile_trees to their corresponding tile atlas
/// by updating the entries with the best available tiles.
pub fn adjust_to_tile_atlas(
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    mut tile_atlases: Query<&mut TileAtlas>,
) {
    for (&(terrain, _view), tile_tree) in tile_trees.iter_mut() {
        let tile_atlas = tile_atlases.get(terrain).unwrap();

        if !tile_tree.dirty && !tile_atlas.changed {
            continue;
        }

        for (tile, entry) in iter::zip(&tile_tree.tiles, &mut tile_tree.data) {
            *entry = tile_atlas.get_best_tile(tile.coordinate);
        }

        tile_tree.dirty = true;
    }

    for mut tile_atlas in &mut tile_atlases {
        tile_atlas.changed = false;
    }
}
