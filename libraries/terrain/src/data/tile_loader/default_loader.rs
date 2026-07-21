use crate::data::{AttachmentData, AttachmentTile, TileAtlas};
use bevy::{
    asset::{AssetId, AssetServer, Assets},
    image::Image,
    platform::collections::HashMap,
    prelude::*,
};
use slab::Slab;

#[derive(Component)]
pub struct DefaultLoader {
    loading_tiles: Slab<super::LoadingTile>,
    /// Maps an in-flight tile's asset id to its slot in `loading_tiles`, so completed/failed
    /// loads can be resolved directly from `AssetEvent`/`AssetLoadFailedEvent` ids instead of
    /// polling every in-flight tile's load state each frame.
    pending: HashMap<AssetId<Image>, usize>,
}

impl Default for DefaultLoader {
    fn default() -> Self {
        Self {
            loading_tiles: Slab::with_capacity(32),
            pending: default(),
        }
    }
}

impl DefaultLoader {
    fn to_load_next(&self, tiles: &mut Vec<AttachmentTile>) -> Option<AttachmentTile> {
        // Todo: tile prioritization goes here
        tiles.pop()
    }

    pub fn finish_loading(
        &mut self,
        atlas: &mut TileAtlas,
        images: &Assets<Image>,
        loaded_ids: &[AssetId<Image>],
        failed_ids: &[AssetId<Image>],
    ) {
        for &id in loaded_ids {
            if let Some(index) = self.pending.remove(&id) {
                let tile = self.loading_tiles.remove(index);
                let image = images.get(id).unwrap();
                let data = AttachmentData::from_bytes(image.data.as_ref().unwrap(), tile.format);
                atlas.tile_loaded(tile.tile, data);
            }
        }

        for &id in failed_ids {
            if let Some(index) = self.pending.remove(&id) {
                self.loading_tiles.remove(index);
            }
        }
    }

    pub fn start_loading(&mut self, atlas: &mut TileAtlas, asset_server: &mut AssetServer) {
        while self.loading_tiles.len() < self.loading_tiles.capacity() {
            if let Some(tile) = self.to_load_next(&mut atlas.to_load) {
                let attachment = &atlas.attachments[&tile.label];

                let path = tile
                    .coordinate
                    .path(&attachment.path.join(String::from(&tile.label)));

                let handle: Handle<Image> = asset_server.load(path);
                let id = handle.id();

                let index = self.loading_tiles.insert(super::LoadingTile {
                    handle,
                    tile,
                    format: attachment.format,
                });
                self.pending.insert(id, index);
            } else {
                break;
            }
        }
    }
}
