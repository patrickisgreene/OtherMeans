use bevy::prelude::*;

use crate::data::{DefaultLoader, TileAtlas};

pub fn finish_loading(
    mut terrains: Query<(&mut TileAtlas, &mut DefaultLoader)>,
    mut asset_server: ResMut<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    for (mut tile_atlas, mut loader) in &mut terrains {
        loader.finish_loading(&mut tile_atlas, &mut asset_server, &mut images);
    }
}
