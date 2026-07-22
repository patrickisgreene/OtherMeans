use crate::{
    config::TerrainConfig,
    data::{
        Attachment, AttachmentData, AttachmentLabel, AttachmentTile, AttachmentTileWithData,
        DefaultLoader, TileTreeEntry,
    },
    math::{TerrainShape, TileCoordinate},
    plugin::TerrainSettings,
    render::TerrainUniform,
};
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::{VisibilityClass, add_visibility_class},
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::{render_resource::*, storage::ShaderBuffer},
    tasks::Task,
};
use big_space::prelude::*;
use std::collections::VecDeque;
// Todo: rename to terrain?
// Todo: consider turning this into an asset

/// A sparse storage of all terrain attachments, which streams data in and out of memory
/// depending on the decisions of the corresponding [`TileTree`]s.
///
/// A tile is considered present and assigned an [`u32`] as soon as it is
/// requested by any tile_tree. Then the tile atlas will start loading all of its attachments
/// by storing the [`TileCoordinate`] (for one frame) in `load_events` for which
/// attachment-loading-systems can listen.
/// Tiles that are not being used by any tile_tree anymore are cached (LRU),
/// until new atlas indices are required.
///
/// The [`u32`] can be used for accessing the attached data in systems by the CPU
/// and in shaders by the GPU.
#[derive(Component)]
#[require(Transform, CellCoord, Visibility, VisibilityClass, DefaultLoader)]
#[component(on_add = add_visibility_class::<TileAtlas>)]
pub struct TileAtlas {
    pub attachments: HashMap<AttachmentLabel, Attachment>, // stores the attachment data
    tile_states: HashMap<TileCoordinate, super::TileState>,
    unused_indices: VecDeque<u32>,
    existing_tiles: HashSet<TileCoordinate>,
    /// Set whenever a tile finishes loading, since that can improve the best-available tile
    /// for a slot even while the view isn't moving. Consumed and reset by `adjust_to_tile_atlas`.
    pub(crate) changed: bool,
    pub uploading_tiles: Vec<AttachmentTileWithData>,
    pub downloading_tiles: Vec<Task<AttachmentTileWithData>>,
    pub to_load: Vec<AttachmentTile>,

    pub lod_count: u32,
    pub min_height: f32,
    pub max_height: f32,
    pub height_scale: f32,
    pub shape: TerrainShape,

    pub terrain_buffer: Handle<ShaderBuffer>,
}

impl TileAtlas {
    /// Creates a new tile_tree from a terrain config.
    pub fn new(
        config: &TerrainConfig,
        buffers: &mut Assets<ShaderBuffer>,
        settings: &TerrainSettings,
    ) -> Self {
        let attachments = config
            .attachments
            .iter()
            .map(|(label, attachment)| (label.clone(), Attachment::new(attachment, &config.path)))
            .collect();

        let terrain_buffer = buffers.add(ShaderBuffer::with_size(
            TerrainUniform::min_size().get() as usize,
            RenderAssetUsages::all(),
        ));

        Self {
            attachments,
            tile_states: default(),
            unused_indices: (0..settings.atlas_size).collect(),
            existing_tiles: HashSet::from_iter(config.tiles.clone()),
            changed: false,
            to_load: default(),
            uploading_tiles: default(),
            downloading_tiles: default(),
            lod_count: config.lod_count,
            min_height: config.min_height,
            max_height: config.max_height,
            height_scale: config.height_scale,
            shape: config.shape,
            terrain_buffer,
        }
    }

    pub(crate) fn get_best_tile(&self, tile_coordinate: TileCoordinate) -> TileTreeEntry {
        let mut best_tile_coordinate = tile_coordinate;

        if !self.existing_tiles.contains(&tile_coordinate) {
            return TileTreeEntry::default();
        }

        loop {
            if best_tile_coordinate == TileCoordinate::INVALID {
                // highest lod is not loaded
                return TileTreeEntry::default();
            }

            if let Some(tile) = self.tile_states.get(&best_tile_coordinate)
                && matches!(tile.state, super::LoadingState::Loaded)
            {
                // found best loaded tile
                return TileTreeEntry {
                    atlas_index: tile.atlas_index,
                    atlas_lod: best_tile_coordinate.lod,
                };
            }

            best_tile_coordinate = best_tile_coordinate
                .parent()
                .unwrap_or(TileCoordinate::INVALID);
        }
    }

    /// Returns the coordinates of all tiles at `lod` that are currently requested by at least
    /// one [`TileTree`](crate::data::TileTree) and have finished loading their attachments.
    /// This is the same definition of "active tile" used internally by [`Self::get_best_tile`].
    pub fn active_tiles_at_lod(&self, lod: u32) -> impl Iterator<Item = TileCoordinate> + '_ {
        self.tile_states.iter().filter_map(move |(&coord, state)| {
            (coord.lod == lod
                && state.requests > 0
                && matches!(state.state, super::LoadingState::Loaded))
            .then_some(coord)
        })
    }

    /// Convenience wrapper around [`Self::active_tiles_at_lod`] using `self.lod_count - 1`.
    pub fn active_tiles_at_highest_lod(&self) -> impl Iterator<Item = TileCoordinate> + '_ {
        self.active_tiles_at_lod(self.lod_count.saturating_sub(1))
    }

    pub(crate) fn tile_loaded(&mut self, tile: AttachmentTile, data: AttachmentData) {
        if let Some(tile_state) = self.tile_states.get_mut(&tile.coordinate) {
            tile_state.state = match tile_state.state {
                super::LoadingState::Loading(1) => super::LoadingState::Loaded,
                super::LoadingState::Loading(n) => super::LoadingState::Loading(n - 1),
                super::LoadingState::Loaded => {
                    panic!("Loaded more attachments, than registered with the tile atlas.")
                }
            };

            if matches!(tile_state.state, super::LoadingState::Loaded) {
                self.changed = true;
            }

            self.uploading_tiles.push(AttachmentTileWithData {
                atlas_index: tile_state.atlas_index,
                label: tile.label,
                data,
            });
        }
    }

    pub fn request_tile(&mut self, tile_coordinate: TileCoordinate) {
        if !self.existing_tiles.contains(&tile_coordinate) {
            return;
        }

        // check if the tile is already present else start loading it
        if let Some(tile) = self.tile_states.get_mut(&tile_coordinate) {
            if tile.requests == 0 {
                // the tile is now used again
                self.unused_indices
                    .retain(|&atlas_index| tile.atlas_index != atlas_index);
            }

            tile.requests += 1;
        } else {
            let atlas_index = self
                .unused_indices
                .pop_front()
                .expect("Atlas out of indices");

            self.tile_states
                .retain(|_, tile| tile.atlas_index != atlas_index); // remove tile if it is still cached

            self.tile_states.insert(
                tile_coordinate,
                super::TileState {
                    requests: 1,
                    state: super::LoadingState::Loading(self.attachments.len() as u32),
                    atlas_index,
                },
            );

            for label in self.attachments.keys() {
                self.to_load.push(AttachmentTile {
                    coordinate: tile_coordinate,
                    label: label.clone(),
                });
            }
        }
    }

    pub fn release_tile(&mut self, tile_coordinate: TileCoordinate) {
        if !self.existing_tiles.contains(&tile_coordinate) {
            return;
        }

        let tile = self.tile_states.get_mut(&tile_coordinate).unwrap();
        tile.requests -= 1;

        if tile.requests == 0 {
            self.unused_indices.push_back(tile.atlas_index);

            // Todo: we should cancel loading tiles, that have not yet started loading and a no longer requested
        }
    }
}
