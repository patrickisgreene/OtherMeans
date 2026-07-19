use crate::data::{AttachmentFormat, AttachmentTile};
use bevy::{asset::Handle, image::Image};

mod default_loader;
pub mod systems;

pub use default_loader::*;

struct LoadingTile {
    handle: Handle<Image>,
    tile: AttachmentTile,
    format: AttachmentFormat,
}
