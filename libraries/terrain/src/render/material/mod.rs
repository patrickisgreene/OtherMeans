use crate::{
    data::tile_atlas::gpu::GpuTileAtlas,
    debug::DebugTerrain,
    render::{
        DrawTerrainCommand, GpuTerrainView, SetTerrainBindGroup, SetTerrainViewBindGroup,
        TerrainItem,
    },
};
use bevy::{
    pbr::{MeshPipelineViewLayoutKey, SetMaterialBindGroup, SetMeshViewBindGroup, ViewKeyCache},
    prelude::*,
    render::{
        render_phase::{
            DrawFunctions, PhaseItemExtraIndex, SetItemPipeline, ViewSortedRenderPhases,
        },
        render_resource::*,
        sync_world::MainEntity,
        view::RetainedViewEntity,
    },
};
use std::hash::Hash;

mod pipeline;
mod pipeline_flags;
mod plugin;

pub use pipeline::*;
pub use pipeline_flags::TerrainPipelineFlags;
pub use plugin::*;

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct TerrainPipelineKey {
    pub flags: TerrainPipelineFlags,
    pub view_layout_key: MeshPipelineViewLayoutKey,
}

/// The draw function of the terrain. It sets the pipeline and the bind groups and then issues the
/// draw call.
pub(crate) type DrawTerrain = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTerrainBindGroup<1>,
    SetTerrainViewBindGroup<2>,
    SetMaterialBindGroup<3>,
    DrawTerrainCommand,
);

/// Queses all terrain entities for rendering via the terrain pipeline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_terrain<M: Material>(
    draw_functions: Res<DrawFunctions<TerrainItem>>,
    debug: Option<Res<DebugTerrain>>,
    pipeline_cache: Res<PipelineCache>,
    terrain_pipeline: Res<TerrainRenderPipeline<M>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TerrainRenderPipeline<M>>>,
    mut terrain_phases: ResMut<ViewSortedRenderPhases<TerrainItem>>,
    terrains: Query<(Entity, &MainEntity, &GpuTileAtlas, &GpuTerrainView)>,
    view_key_cache: Res<ViewKeyCache>,
    mut views: Query<(MainEntity, &Msaa)>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_function = draw_functions.read().get_id::<DrawTerrain>().unwrap();

    for (view, msaa) in &mut views {
        let retained_view_entity = RetainedViewEntity {
            main_entity: view.into(),
            auxiliary_entity: Entity::PLACEHOLDER.into(),
            subview_index: 0,
        };

        let Some(terrain_phase) = terrain_phases.get_mut(&retained_view_entity) else {
            continue;
        };

        let view_layout_key = view_key_cache
            .get(&retained_view_entity)
            .copied()
            .map(MeshPipelineViewLayoutKey::from)
            .unwrap_or(MeshPipelineViewLayoutKey::empty());

        for (render_entity, main_entity, gpu_tile_atlas, gpu_terrain_view) in &terrains {
            let mut flags = TerrainPipelineFlags::from_msaa_samples(msaa.samples());
            if gpu_tile_atlas.is_spherical {
                flags |= TerrainPipelineFlags::SPHERICAL;
            }

            if let Some(debug) = &debug {
                flags |= TerrainPipelineFlags::from_debug(debug);
            } else {
                flags |= TerrainPipelineFlags::LIGHTING
                    | TerrainPipelineFlags::MORPH
                    | TerrainPipelineFlags::BLEND
                    | TerrainPipelineFlags::SAMPLE_GRAD;
            }

            let key = TerrainPipelineKey {
                flags,
                view_layout_key,
            };

            let pipeline = pipelines.specialize(&pipeline_cache, &terrain_pipeline, key);

            terrain_phase.add_transient(TerrainItem {
                representative_entity: (render_entity, *main_entity),
                draw_function,
                pipeline,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                order: gpu_terrain_view.order,
            })
        }
    }
}
