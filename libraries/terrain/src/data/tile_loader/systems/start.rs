use bevy::prelude::*;

use crate::data::{DefaultLoader, TileAtlas};

pub fn start_loading(
    mut terrains: Query<(&mut TileAtlas, &mut DefaultLoader)>,
    mut asset_server: ResMut<AssetServer>,
) {
    for (mut tile_atlas, mut loader) in &mut terrains {
        loader.start_loading(&mut tile_atlas, &mut asset_server);
    }
}
