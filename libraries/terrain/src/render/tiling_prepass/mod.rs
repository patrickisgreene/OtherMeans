use crate::{
    config::TerrainComponents,
    debug::DebugTerrain,
    render::{GpuTerrain, GpuTerrainView},
    view::TerrainViewComponents,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderContext},
};

mod pipeline_key;
mod pipelines;
pub mod systems;

pub use pipeline_key::TilingPrepassPipelineKey;
pub use pipelines::TerrainTilingPrepassPipelines;

pub struct TilingPrepassItem {
    refine_tiles_pipeline: CachedComputePipelineId,
    prepare_root_pipeline: CachedComputePipelineId,
    prepare_next_pipeline: CachedComputePipelineId,
    prepare_render_pipeline: CachedComputePipelineId,
}

impl TilingPrepassItem {
    fn pipelines<'a>(
        &'a self,
        pipeline_cache: &'a PipelineCache,
    ) -> Option<(
        &'a ComputePipeline,
        &'a ComputePipeline,
        &'a ComputePipeline,
        &'a ComputePipeline,
    )> {
        Some((
            pipeline_cache.get_compute_pipeline(self.refine_tiles_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.prepare_root_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.prepare_next_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.prepare_render_pipeline)?,
        ))
    }
}

pub fn tiling_prepass(world: &World, mut ctx: RenderContext) {
    let prepass_items = world.resource::<TerrainViewComponents<TilingPrepassItem>>();
    let pipeline_cache = world.resource::<PipelineCache>();
    let gpu_terrains = world.resource::<TerrainComponents<GpuTerrain>>();
    let gpu_terrain_views = world.resource::<TerrainViewComponents<GpuTerrainView>>();
    let debug = world.get_resource::<DebugTerrain>();

    if debug.map(|debug| debug.freeze).unwrap_or(false) {
        return;
    }

    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());

    for (&(terrain, view), prepass_item) in prepass_items.iter() {
        let Some((
            refine_tiles_pipeline,
            prepare_root_pipeline,
            prepare_next_pipeline,
            prepare_render_pipeline,
        )) = prepass_item.pipelines(pipeline_cache)
        else {
            continue;
        };

        let gpu_terrain = &gpu_terrains[&terrain];
        let gpu_terrain_view = &gpu_terrain_views[&(terrain, view)];

        let Some(terrain_bind_group) = &gpu_terrain.terrain_bind_group else {
            continue;
        };
        let Some(prepass_view_bind_group) = &gpu_terrain_view.prepass_view_bind_group else {
            continue;
        };
        let Some(indirect_bind_group) = &gpu_terrain_view.indirect_bind_group else {
            continue;
        };

        pass.set_bind_group(0, prepass_view_bind_group, &[]);
        pass.set_bind_group(1, terrain_bind_group, &[]);
        pass.set_bind_group(2, indirect_bind_group, &[]);

        pass.set_pipeline(prepare_root_pipeline);
        pass.dispatch_workgroups(1, 1, 1);

        for _ in 0..gpu_terrain_view.refinement_count {
            pass.set_pipeline(refine_tiles_pipeline);
            pass.dispatch_workgroups_indirect(&gpu_terrain_view.indirect_buffer, 0);

            pass.set_pipeline(prepare_next_pipeline);
            pass.dispatch_workgroups(1, 1, 1);
        }

        pass.set_pipeline(refine_tiles_pipeline);
        pass.dispatch_workgroups_indirect(&gpu_terrain_view.indirect_buffer, 0);

        pass.set_pipeline(prepare_render_pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
