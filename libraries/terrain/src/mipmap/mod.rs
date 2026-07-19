use crate::{
    config::TerrainComponents,
    data::{attachment::AttachmentFormat, tile_atlas::gpu::GpuTileAtlas},
};
use bevy::{
    prelude::*,
    render::{
        render_resource::{binding_types::*, *},
        renderer::RenderContext,
    },
};

mod pipeline_key;
mod pipelines;

pub(crate) use self::pipeline_key::*;
pub(crate) use self::pipelines::*;

pub(crate) fn create_mip_layout(format: AttachmentFormat) -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "mip_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<u32>(false), // atlas_index
                texture_2d_array(TextureSampleType::Float { filterable: true }), // parent
                texture_storage_2d_array(
                    format.processing_format(),
                    StorageTextureAccess::WriteOnly,
                ), // child
            ),
        ),
    )
}

pub(crate) fn mip_prepass(world: &World, mut ctx: RenderContext) {
    let pipeline_cache = world.resource::<PipelineCache>();
    let gpu_tile_atlases = world.resource::<TerrainComponents<GpuTileAtlas>>();

    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());

    for gpu_tile_atlas in gpu_tile_atlases.values() {
        gpu_tile_atlas.generate_mip(&mut pass, pipeline_cache);
    }
}
