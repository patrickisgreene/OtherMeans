use crate::{config::TerrainComponents, data::tile_atlas::gpu::GpuTileAtlas};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderContext},
};

mod mip_layout;
mod pipeline_key;
mod pipelines;

pub(crate) use mip_layout::AsMipLayout;
pub(crate) use pipeline_key::*;
pub(crate) use pipelines::*;

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

pub(crate) const MIP_SHADER: &str = "embedded://terrain/shaders/mipmap.wgsl";
