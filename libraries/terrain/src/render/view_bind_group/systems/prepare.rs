use bevy::{
    ecs::system::StaticSystemParam,
    prelude::*,
    render::{
        render_resource::{AsBindGroup, PipelineCache},
        renderer::RenderDevice,
    },
};

use crate::{
    render::{
        GpuTerrainView, IndirectBindGroup, PrepassViewBindGroup, TerrainTilingPrepassPipelines,
        TerrainViewBindGroup,
    },
    view::TerrainViewComponents,
};

pub fn prepare_terrain_view(
    device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipeline: Res<TerrainTilingPrepassPipelines>,
    mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
    mut param: StaticSystemParam<<TerrainViewBindGroup as AsBindGroup>::Param>,
) {
    for gpu_terrain_view in &mut gpu_terrain_views.values_mut() {
        // Todo: be smarter about bind group recreation
        let bind_group = gpu_terrain_view.terrain_view.as_bind_group(
            &prepass_pipeline.terrain_view_layout,
            &device,
            &pipeline_cache,
            &mut param,
        );
        gpu_terrain_view.terrain_view_bind_group = bind_group.ok().map(|b| b.bind_group);
    }
}

pub fn prepare_indirect(
    device: Res<RenderDevice>,
    prepass_pipeline: Res<TerrainTilingPrepassPipelines>,
    pipeline_cache: Res<PipelineCache>,
    mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
    mut param: StaticSystemParam<<IndirectBindGroup as AsBindGroup>::Param>,
) {
    for gpu_terrain_view in &mut gpu_terrain_views.values_mut() {
        let bind_group = &mut gpu_terrain_view.indirect_bind_group;

        if bind_group.is_none() {
            *bind_group = gpu_terrain_view
                .indirect
                .as_bind_group(
                    &prepass_pipeline.indirect_layout,
                    &device,
                    &pipeline_cache,
                    &mut param,
                )
                .ok()
                .map(|b| b.bind_group);
        }
    }
}

pub fn prepare_refine_tiles(
    device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipeline: Res<TerrainTilingPrepassPipelines>,
    mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
    mut param: StaticSystemParam<<PrepassViewBindGroup as AsBindGroup>::Param>,
) {
    for gpu_terrain_view in gpu_terrain_views.values_mut() {
        // Todo: be smarter about bind group recreation
        let bind_group = gpu_terrain_view.prepass_view.as_bind_group(
            &prepass_pipeline.prepass_view_layout,
            &device,
            &pipeline_cache,
            &mut param,
        );
        gpu_terrain_view.prepass_view_bind_group = bind_group.ok().map(|b| b.bind_group);
    }
}
