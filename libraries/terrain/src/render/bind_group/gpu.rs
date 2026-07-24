use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BufferUsages, FilterMode, MipmapFilterMode, Sampler, SamplerDescriptor,
            TextureUsages, TextureView, TextureViewDescriptor,
        },
        renderer::RenderDevice,
        storage::ShaderBuffer,
        texture::FallbackImage,
    },
};

use crate::{
    data::{TileAtlas, tile_atlas::gpu::GpuTileAtlas},
    render::AttachmentUniform,
    util::GpuBuffer,
};

#[derive(Component)]
pub struct GpuTerrain {
    pub terrain_bind_group: Option<BindGroup>,

    pub terrain_buffer: Handle<ShaderBuffer>,
    pub atlas_sampler: Sampler,
    pub attachment_textures: [TextureView; 8],
    pub attachment_buffer: GpuBuffer<AttachmentUniform>,
}

impl GpuTerrain {
    pub fn new(
        device: &RenderDevice,
        fallback_image: &FallbackImage,
        tile_atlas: &TileAtlas,
        gpu_tile_atlas: &GpuTileAtlas,
    ) -> Self {
        let attachment_buffer = GpuBuffer::create(
            device,
            &AttachmentUniform::new(gpu_tile_atlas),
            BufferUsages::UNIFORM,
        );

        let attachment_textures = std::array::from_fn(|i| {
            gpu_tile_atlas
                .attachments
                .iter()
                .find(|(_, attachment)| attachment.index == i)
                .map_or(
                    fallback_image.d2_array.texture_view.clone(),
                    |(_, attachment)| {
                        attachment
                            .atlas_texture
                            .create_view(&TextureViewDescriptor {
                                format: Some(attachment.buffer_info.format.render_format()),
                                usage: Some(TextureUsages::TEXTURE_BINDING),
                                ..default()
                            })
                    },
                )
        });

        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            anisotropy_clamp: 16, // Todo: make this customisable
            ..default()
        });

        Self {
            terrain_buffer: tile_atlas.terrain_buffer.clone(),
            attachment_buffer,
            atlas_sampler,
            attachment_textures,
            terrain_bind_group: None,
        }
    }
}
