use bevy::{
    prelude::*,
    render::render_resource::{PipelineCache, SpecializedComputePipelines},
};

use crate::{
    config::TerrainComponents,
    data::tile_atlas::gpu::GpuTileAtlas,
    debug::DebugTerrain,
    render::{
        GpuTerrainView, TerrainTilingPrepassPipelines, TilingPrepassItem, TilingPrepassPipelineKey,
    },
    view::TerrainViewComponents,
};

pub fn queue_tiling_prepass(
    debug: Option<Res<DebugTerrain>>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipelines: ResMut<TerrainTilingPrepassPipelines>,
    mut pipelines: ResMut<SpecializedComputePipelines<TerrainTilingPrepassPipelines>>,
    mut prepass_items: ResMut<TerrainViewComponents<TilingPrepassItem>>,
    gpu_terrain_views: Res<TerrainViewComponents<GpuTerrainView>>,
    gpu_tile_atlases: Res<TerrainComponents<GpuTileAtlas>>,
) {
    for &(terrain, view) in gpu_terrain_views.keys() {
        let gpu_tile_atlas = &gpu_tile_atlases[&terrain];

        let mut key = TilingPrepassPipelineKey::NONE;

        if gpu_tile_atlas.is_spherical {
            key |= TilingPrepassPipelineKey::SPHERICAL;
        }

        if let Some(debug) = &debug {
            key |= TilingPrepassPipelineKey::from_debug(debug);
        }

        let refine_tiles_pipeline = pipelines.specialize(
            &pipeline_cache,
            &prepass_pipelines,
            key | TilingPrepassPipelineKey::REFINE_TILES,
        );
        let prepare_root_pipeline = pipelines.specialize(
            &pipeline_cache,
            &prepass_pipelines,
            key | TilingPrepassPipelineKey::PREPARE_ROOT,
        );
        let prepare_next_pipeline = pipelines.specialize(
            &pipeline_cache,
            &prepass_pipelines,
            key | TilingPrepassPipelineKey::PREPARE_NEXT,
        );
        let prepare_render_pipeline = pipelines.specialize(
            &pipeline_cache,
            &prepass_pipelines,
            key | TilingPrepassPipelineKey::PREPARE_RENDER,
        );

        prepass_items.insert(
            (terrain, view),
            TilingPrepassItem {
                refine_tiles_pipeline,
                prepare_root_pipeline,
                prepare_next_pipeline,
                prepare_render_pipeline,
            },
        );
    }
}
