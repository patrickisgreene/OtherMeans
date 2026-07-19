use super::AttachmentFormat;

use serde::{Deserialize, Serialize};

/// Configures an attachment.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttachmentConfig {
    /// The name of the attachment.
    pub texture_size: u32,
    /// The overlapping border size around the tile, used to prevent sampling artifacts.
    pub border_size: u32,
    pub mip_level_count: u32,
    pub mask: bool,
    /// The format of the attachment.
    pub format: AttachmentFormat,
}

impl Default for AttachmentConfig {
    fn default() -> Self {
        Self {
            texture_size: 512,
            border_size: 2,
            mip_level_count: 2,
            mask: false,
            format: AttachmentFormat::Rgba8U,
        }
    }
}

impl AttachmentConfig {
    pub fn center_size(&self) -> u32 {
        self.texture_size - 2 * self.border_size
    }

    pub fn offset_size(&self) -> u32 {
        self.texture_size - self.border_size
    }
}
