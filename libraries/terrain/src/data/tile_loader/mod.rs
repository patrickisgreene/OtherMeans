use crate::data::{AttachmentFormat, AttachmentTile};
use bevy::{asset::Handle, image::Image};

mod default_loader;
pub mod systems;

pub use default_loader::*;

struct LoadingTile {
    // Never read directly - keeps a strong handle alive so the asset server doesn't drop the
    // load while it's in flight; completion is detected via AssetEvent/AssetLoadFailedEvent ids.
    #[allow(dead_code)]
    handle: Handle<Image>,
    tile: AttachmentTile,
    format: AttachmentFormat,
}
