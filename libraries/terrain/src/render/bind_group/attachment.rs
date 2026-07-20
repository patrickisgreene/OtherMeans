use bevy::render::render_resource::ShaderType;

use crate::data::{gpu::GpuAttachment, tile_atlas::gpu::GpuTileAtlas};

#[derive(Default, ShaderType)]
struct AttachmentConfig {
    texture_size: f32,
    center_size: f32,
    scale: f32,
    offset: f32,
    mask: u32,
    padding1: u32,
    padding2: u32,
    padding3: u32,
}

impl AttachmentConfig {
    pub fn new(attachment: &GpuAttachment) -> Self {
        Self {
            center_size: attachment.buffer_info.center_size as f32,
            texture_size: attachment.buffer_info.texture_size as f32,
            scale: attachment.buffer_info.center_size as f32
                / attachment.buffer_info.texture_size as f32,
            offset: attachment.buffer_info.border_size as f32
                / attachment.buffer_info.texture_size as f32,
            mask: attachment.buffer_info.mask as u32,
            padding1: 0,
            padding2: 0,
            padding3: 0,
        }
    }
}

#[derive(Default, ShaderType)]
pub struct AttachmentUniform {
    attachments: [AttachmentConfig; 8],
}

impl AttachmentUniform {
    pub fn new(tile_atlas: &GpuTileAtlas) -> Self {
        Self {
            attachments: std::array::from_fn(|i| {
                tile_atlas
                    .attachments
                    .iter()
                    .find(|(_, attachment)| attachment.index == i)
                    .map_or(AttachmentConfig::default(), |(_, attachment)| {
                        AttachmentConfig::new(attachment)
                    })
            }),
        }
    }
}
