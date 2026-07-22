use bevy::{
    core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d},
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::{SetMeshViewBindGroup, ViewKeyCache},
    prelude::*,
    render::{
        render_phase::{
            DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
            SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity},
        view::ExtractedView,
        Extract,
    },
};

use crate::instances::{BuildingInstances, InstanceData};
use crate::render::origin::BuildingTileOrigin;
use crate::render::pipeline::{BuildingsPipeline, BuildingsPipelineKey, CubeVertexBuffer};

/// Extracts newly-spawned (or, in principle, changed) [`BuildingInstances`] into the render
/// world. Gated on `Changed<>` in the main world - unlike `ExtractComponentPlugin`, which
/// queries and re-clones *every* matching entity *every frame* regardless of whether it
/// changed. Since `BuildingInstances` is only ever inserted once per tile (never mutated
/// afterward), this fires exactly once per tile's lifetime.
pub fn extract_building_instances(
    mut commands: Commands,
    query: Extract<Query<(RenderEntity, &BuildingInstances), Changed<BuildingInstances>>>,
) {
    for (entity, instances) in &query {
        commands.entity(entity).insert(instances.clone());
    }
}

/// A single, persistent, reused GPU buffer holding every currently-active tile's building
/// instances, merged and baked to absolute (camera-relative) positions each frame. Replaces a
/// separate freshly-allocated buffer per tile - `RawBufferVec::write_buffer` reuses the
/// existing GPU buffer whenever the merged data still fits, only reallocating when it grows.
#[derive(Resource)]
pub struct MergedBuildingInstances(pub RawBufferVec<InstanceData>);

impl Default for MergedBuildingInstances {
    fn default() -> Self {
        Self(RawBufferVec::new(BufferUsages::VERTEX))
    }
}

/// A persistent placeholder entity used as the `Transparent3d` phase item's associated entity,
/// since buildings are no longer rendered per-tile-entity - mirrors
/// `bevy_gizmos_render`'s `LineGizmoEntities` pattern for phase items with no natural backing
/// entity.
#[derive(Resource)]
pub struct BuildingsRendererEntity(pub Entity);

impl FromWorld for BuildingsRendererEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(Name::new("BuildingsRenderer")).id())
    }
}

pub fn prepare_merged_building_buffer(
    mut merged: ResMut<MergedBuildingInstances>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    tiles: Query<(&BuildingInstances, &BuildingTileOrigin)>,
) {
    merged.0.values_mut().clear();

    for (instances, origin) in &tiles {
        for instance in &instances.0 {
            let mut baked = *instance;
            baked.position_and_footprint[0] += origin.translation.x;
            baked.position_and_footprint[1] += origin.translation.y;
            baked.position_and_footprint[2] += origin.translation.z;
            merged.0.push(baked);
        }
    }

    merged.0.write_buffer(&render_device, &render_queue);
}

pub struct DrawBuildingsInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawBuildingsInstanced {
    type Param = (SRes<CubeVertexBuffer>, SRes<MergedBuildingInstances>);
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (cube, merged): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let merged = merged.into_inner();
        let Some(buffer) = merged.0.buffer() else {
            return RenderCommandResult::Skip;
        };
        let cube = cube.into_inner();

        pass.set_vertex_buffer(0, cube.buffer.slice(..));
        pass.set_vertex_buffer(1, buffer.slice(..));
        pass.draw(0..cube.vertex_count, 0..merged.0.len() as u32);

        RenderCommandResult::Success
    }
}

pub type DrawBuildings = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    DrawBuildingsInstanced,
);

pub fn queue_buildings(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    buildings_pipeline: Res<BuildingsPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BuildingsPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_key_cache: Res<ViewKeyCache>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    merged: Res<MergedBuildingInstances>,
    renderer_entity: Res<BuildingsRendererEntity>,
) {
    if merged.0.is_empty() {
        return;
    }

    let draw_function = draw_functions.read().get_id::<DrawBuildings>().unwrap();
    let entity = renderer_entity.0;
    let main_entity = MainEntity::from(entity);

    for view in &views {
        let Some(transparent_phase) =
            transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &buildings_pipeline,
            BuildingsPipelineKey { view_key },
        );

        transparent_phase.add_transient(Transparent3d {
            sorting_info: TransparentSortingInfo3d::AlwaysOnTop,
            distance: 0.0,
            entity: (entity, main_entity),
            draw_function,
            pipeline,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            indexed: false,
        });
    }
}
