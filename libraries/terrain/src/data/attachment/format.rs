use bevy::render::render_resource::TextureFormat;
use serde::{Deserialize, Serialize};
use std::{fmt::Error, str::FromStr};

/// The data format of an attachment.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachmentFormat {
    /// Three channels  8 bit unsigned integer
    Rgb8U,
    /// Four channels  8 bit unsigned integer
    Rgba8U,
    /// One channel  16 bit unsigned integer
    R16U,
    /// One channel  16 bit integer
    R16I,
    /// Two channels 16 bit unsigned integer
    Rg16U,
    /// One channel 32 bit float
    R32F,
}

impl FromStr for AttachmentFormat {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "rgb8u" => Ok(Self::Rgb8U),
            "rgba8u" => Ok(Self::Rgba8U),
            "r16u" => Ok(Self::R16U),
            "r16i" => Ok(Self::R16I),
            "r32f" => Ok(Self::R32F),
            _ => Err(Error),
        }
    }
}

impl AttachmentFormat {
    pub const ALL: [Self; 6] = [
        Self::Rgb8U,
        Self::Rgba8U,
        Self::R16U,
        Self::R16I,
        Self::Rg16U,
        Self::R32F,
    ];

    pub(crate) fn render_format(self) -> TextureFormat {
        match self {
            AttachmentFormat::Rgb8U => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::Rgba8U => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::R16U => TextureFormat::R16Unorm,
            AttachmentFormat::R16I => TextureFormat::R16Snorm,
            AttachmentFormat::Rg16U => TextureFormat::Rg16Unorm,
            AttachmentFormat::R32F => TextureFormat::R32Float,
        }
    }

    pub(crate) fn processing_format(self) -> TextureFormat {
        match self {
            AttachmentFormat::Rgb8U => TextureFormat::Rgba8Unorm,
            AttachmentFormat::Rgba8U => TextureFormat::Rgba8Unorm,
            AttachmentFormat::R16U => TextureFormat::R16Uint,
            AttachmentFormat::R16I => TextureFormat::R16Uint,
            AttachmentFormat::Rg16U => TextureFormat::Rg16Uint,
            _ => self.render_format(),
        }
    }

    pub(crate) fn pixel_size(self) -> u32 {
        match self {
            AttachmentFormat::Rgb8U => 4,
            AttachmentFormat::Rgba8U => 4,
            AttachmentFormat::R16U => 2,
            AttachmentFormat::R16I => 2,
            AttachmentFormat::Rg16U => 4,
            AttachmentFormat::R32F => 4,
        }
    }
}
