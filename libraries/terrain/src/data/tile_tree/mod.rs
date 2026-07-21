use crate::{
    data::{INVALID_ATLAS_INDEX, INVALID_LOD},
    math::TileCoordinate,
};
use bevy::{prelude::*, render::render_resource::ShaderType};

mod height_readback;
pub mod systems;
#[allow(clippy::module_inception)]
mod tile_tree;

pub use height_readback::*;
pub use tile_tree::*;

/// The current state of a tile of a [`TileTree`].
///
/// This indicates, whether or not the tile should be loaded into the [`TileAtlas`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    /// The tile should be loaded.
    Requested,
    /// The tile does not have to be loaded.
    Released,
}

/// The internal representation of a tile in a [`TileTree`].
pub struct TileState {
    /// The current tile coordinate at the tile_tree position.
    coordinate: TileCoordinate,
    /// Indicates, whether the tile is currently demanded or released.
    state: RequestState,
}

impl Default for TileState {
    fn default() -> Self {
        Self {
            coordinate: TileCoordinate::INVALID,
            state: RequestState::Released,
        }
    }
}

/// An entry of the [`TileTree`], used to access the best currently loaded tile
/// of the [`TileAtlas`] on the CPU.
///
/// These entries are synced each frame with their equivalent representations in the
/// [`GpuTileTree`](super::gpu_tile_tree::GpuTileTree) for access on the GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TileTreeEntry {
    /// The atlas index of the best entry.
    pub(crate) atlas_index: u32,
    /// The atlas lod of the best entry.
    pub(crate) atlas_lod: u32,
}

impl Default for TileTreeEntry {
    fn default() -> Self {
        Self {
            atlas_index: INVALID_ATLAS_INDEX,
            atlas_lod: INVALID_LOD,
        }
    }
}

#[derive(Component)]
pub struct TerrainViewKey(pub (Entity, Entity));
