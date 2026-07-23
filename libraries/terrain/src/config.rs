//! Types for configuring terrains.
//!

use crate::{
    data::{AttachmentConfig, AttachmentLabel},
    math::{TerrainShape, TileCoordinate},
};
use bevy::{ecs::entity::hash_map::EntityHashMap, platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Resource that stores components that are associated to a terrain entity.
/// This is used to persist components in the render world.
#[derive(Deref, DerefMut, Resource)]
pub struct TerrainComponents<C>(EntityHashMap<C>);

impl<C> Default for TerrainComponents<C> {
    fn default() -> Self {
        Self(default())
    }
}

/// The configuration of a terrain.
///
/// Here you can define all fundamental parameters of the terrain.
#[derive(Serialize, Deserialize, Asset, TypePath, Debug, Clone)]
pub struct TerrainConfig {
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    pub shape: TerrainShape,
    /// The count of level of detail layers.
    pub lod_count: u32,
    pub min_height: f32,
    pub max_height: f32,
    /// Converts `min_height`/`max_height` and the height attachment's sampled values (both
    /// stored in whatever unit the source data used) into real-world metres for displacement
    /// and LOD/AABB bounds - see `render::bind_group::TerrainUniform::new` and
    /// `shaders::attachments::sample_height`. 1.0 is correct when the source height data is
    /// already in metres (e.g. a real elevation raster); a preprocessing pipeline that
    /// normalizes height into another range (e.g. to keep 0.0 an unambiguous sentinel, see
    /// `resources/earth/scripts/heightmap.sh`) needs this set to the real metres-per-unit
    /// factor instead, or displacement ends up scaled down by that same normalization.
    #[serde(default = "default_height_scale")]
    pub height_scale: f32,
    /// The attachments of the terrain.
    pub attachments: HashMap<AttachmentLabel, AttachmentConfig>,
    /// The tiles of the terrain.
    pub tiles: Vec<TileCoordinate>,
}

fn default_height_scale() -> f32 {
    10000.0
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            shape: TerrainShape::Plane { side_length: 1.0 },
            lod_count: 1,
            min_height: 0.0,
            max_height: 1.0,
            height_scale: default_height_scale(),
            path: default(),
            tiles: default(),
            attachments: default(),
        }
    }
}

impl TerrainConfig {
    pub fn add_attachment(
        &mut self,
        label: AttachmentLabel,
        attachment: AttachmentConfig,
    ) -> &mut Self {
        self.attachments.insert(label, attachment);
        self
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let encoded = fs::read_to_string(path)?;
        Ok(ron::from_str(&encoded)?)
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let encoded = ron::ser::to_string_pretty(self, default())?;
        Ok(fs::write(path, encoded)?)
    }
}
