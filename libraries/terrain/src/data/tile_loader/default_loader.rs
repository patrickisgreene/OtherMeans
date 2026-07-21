use crate::data::{AttachmentData, AttachmentTile, TileAtlas};
use bevy::{
    asset::{AssetServer, Assets},
    image::Image,
    prelude::*,
};
use slab::Slab;

#[derive(Component)]
pub struct DefaultLoader {
    loading_tiles: Slab<super::LoadingTile>,
}

impl Default for DefaultLoader {
    fn default() -> Self {
        Self {
            loading_tiles: Slab::with_capacity(32),
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
        asset_server: &mut AssetServer,
        images: &mut Assets<Image>,
    ) {
        self.loading_tiles.retain(|_, tile| {
            if asset_server.is_loaded(tile.handle.id()) {
                let image = images.get(tile.handle.id()).unwrap();
                let data = AttachmentData::from_bytes(image.data.as_ref().unwrap(), tile.format);
                atlas.tile_loaded(tile.tile.clone(), data);

                false
            } else {
                !asset_server.load_state(tile.handle.id()).is_failed()
            }
        });
    }

    pub fn start_loading(&mut self, atlas: &mut TileAtlas, asset_server: &mut AssetServer) {
        while self.loading_tiles.len() < self.loading_tiles.capacity() {
            if let Some(tile) = self.to_load_next(&mut atlas.to_load) {
                let attachment = &atlas.attachments[&tile.label];

                let path = tile
                    .coordinate
                    .path(&attachment.path.join(String::from(&tile.label)));

                self.loading_tiles.insert(super::LoadingTile {
                    handle: asset_server.load(path),
                    tile,
                    format: attachment.format,
                });
            } else {
                break;
            }
        }
    }
}
