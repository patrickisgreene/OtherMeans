pub mod gpu;
pub mod systems;
mod tile_atlas;

pub use tile_atlas::*;

/// The current state of a tile of a [`TileAtlas`].
///
/// This indicates, whether the tile is loading or loaded and ready to be used.
#[derive(Clone, Copy, Debug)]
enum LoadingState {
    /// The tile is loading, but can not be used yet.
    Loading(u32),
    /// The tile is loaded and can be used.
    Loaded,
}

/// The internal representation of a present tile in a [`TileAtlas`].
struct TileState {
    /// Indicates whether or not the tile is loading or loaded.
    state: LoadingState,
    /// The index of the tile inside the atlas.
    atlas_index: u32,
    /// The count of [`TileTrees`] that have requested this tile.
    requests: u32,
}
