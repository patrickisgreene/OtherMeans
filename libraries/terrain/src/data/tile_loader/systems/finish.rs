use bevy::{
    asset::{AssetId, AssetLoadFailedEvent},
    prelude::*,
};

use crate::data::{DefaultLoader, TileAtlas};

pub fn finish_loading(
    mut terrains: Query<(&mut TileAtlas, &mut DefaultLoader)>,
    images: Res<Assets<Image>>,
    mut loaded_events: MessageReader<AssetEvent<Image>>,
    mut failed_events: MessageReader<AssetLoadFailedEvent<Image>>,
) {
    let loaded_ids: Vec<AssetId<Image>> = loaded_events
        .read()
        .filter_map(|event| match event {
            AssetEvent::LoadedWithDependencies { id } => Some(*id),
            _ => None,
        })
        .collect();
    let failed_ids: Vec<AssetId<Image>> = failed_events.read().map(|event| event.id).collect();

    for (mut tile_atlas, mut loader) in &mut terrains {
        loader.finish_loading(&mut tile_atlas, &images, &loaded_ids, &failed_ids);
    }
}
