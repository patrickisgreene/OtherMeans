use crate::data::AttachmentConfig;
use crate::data::AttachmentFormat;
use crate::data::AttachmentLabel;
use crate::math::TileCoordinate;
use bytemuck::cast_slice;
use itertools::Itertools;
use std::path::{Component, Path, PathBuf};

#[derive(Clone)]
pub enum AttachmentData {
    /// Three channels  8 bit
    // Rgb8(Vec<(u8, u8, u8)>), Can not be represented currently
    /// Four  channels  8 bit
    Rgba8U(Vec<[u8; 4]>),
    /// One   channel  16 bit
    R16U(Vec<u16>),
    /// One   channel  16 bit
    R16I(Vec<i16>),
    /// Two   channels 16 bit
    Rg16U(Vec<[u16; 2]>),
    R32F(Vec<f32>),
}

impl AttachmentData {
    pub(crate) fn from_bytes(data: &[u8], format: AttachmentFormat) -> Self {
        match format {
            AttachmentFormat::Rgb8U => Self::Rgba8U(
                data.chunks(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2], 255])
                    .collect_vec(),
            ),
            AttachmentFormat::Rgba8U => Self::Rgba8U(cast_slice(data).to_vec()),
            AttachmentFormat::R16U => Self::R16U(cast_slice(data).to_vec()),
            AttachmentFormat::R16I => Self::R16I(cast_slice(data).to_vec()),
            AttachmentFormat::Rg16U => Self::Rg16U(cast_slice(data).to_vec()),
            AttachmentFormat::R32F => Self::R32F(cast_slice(data).to_vec()),
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            AttachmentData::Rgba8U(data) => cast_slice(data),
            AttachmentData::R16U(data) => cast_slice(data),
            AttachmentData::R16I(data) => cast_slice(data),
            AttachmentData::Rg16U(data) => cast_slice(data),
            AttachmentData::R32F(data) => cast_slice(data),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AttachmentTile {
    pub(crate) coordinate: TileCoordinate,
    pub(crate) label: AttachmentLabel,
}

#[derive(Clone)]
pub struct AttachmentTileWithData {
    pub(crate) atlas_index: u32,
    pub(crate) label: AttachmentLabel,
    pub(crate) data: AttachmentData,
}

/// An attachment of a [`TileAtlas`].
pub struct Attachment {
    pub(crate) path: PathBuf,
    pub(crate) texture_size: u32,
    pub(crate) center_size: u32,
    pub(crate) border_size: u32,
    pub(crate) mip_level_count: u32,
    pub(crate) format: AttachmentFormat,
    pub(crate) mask: bool,
}

/// Bevy resolves asset-load paths relative to the configured asset root (e.g. `example/assets`),
/// but `config.tc.ron`'s stored `path` may be an absolute filesystem path (whatever
/// `--terrain-path` was passed to terrain-preprocess with) - joining that in unchanged would
/// duplicate the asset root in the final load path. Since the terrain's own tiles always live
/// somewhere under the project's `assets` directory, find that component (wherever it falls,
/// relative or absolute) and keep only what comes after it.
pub(crate) fn asset_relative_path(path: &Path) -> PathBuf {
    let mut components = path.components();
    if components
        .by_ref()
        .any(|component| component == Component::Normal("assets".as_ref()))
    {
        components.as_path().to_path_buf()
    } else {
        path.to_path_buf()
    }
}

impl Attachment {
    /// Asset-relative base path tiles of this attachment are loaded from (join with
    /// [`TileCoordinate::path`] for a specific tile's file).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Full side length (in texels) of one tile's stored image, including the border padding on
    /// both sides - see [`Self::border_size`]/[`Self::center_size`].
    pub fn texture_size(&self) -> u32 {
        self.texture_size
    }

    /// Side length (in texels) of a tile's actual (non-overlapping) content, i.e.
    /// `texture_size - 2 * border_size`.
    pub fn center_size(&self) -> u32 {
        self.center_size
    }

    /// Width (in texels) of the overlapping border baked into each tile's stored image, used to
    /// avoid sampling artifacts at tile edges - see `compute_sample_uv` in
    /// `shaders/attachments.wgsl` for how this maps a tile-local UV into the padded texture.
    pub fn border_size(&self) -> u32 {
        self.border_size
    }

    pub(crate) fn new(config: &AttachmentConfig, path: &str) -> Self {
        let path = asset_relative_path(Path::new(path));

        Self {
            path,
            texture_size: config.texture_size,
            center_size: config.center_size(),
            border_size: config.border_size,
            mip_level_count: config.mip_level_count,
            format: config.format,
            mask: config.mask,
        }
    }
}
