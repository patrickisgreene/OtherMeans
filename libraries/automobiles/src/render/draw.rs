use bevy::{
    core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d},
    ecs::{
        query::ROQueryItem,
        system::{SystemParamItem, lifetimeless::SRes},
    },
    pbr::{SetMeshViewBindGroup, ViewKeyCache},
    prelude::*,
    render::{
        Extract,
        render_phase::{
            DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
            SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity},
        view::ExtractedView,
    },
};

use crate::instances::{AutomobilesInstances, InstanceData};
use crate::render::origin::AutomobilesTileOrigin;
use crate::render::pipeline::{AutomobilesPipeline, AutomobilesPipelineKey, TruckMeshBuffer};
use crate::render::time::SetRenderParamsBindGroup;

/// Extracts newly-spawned (or, in principle, changed) [`AutomobilesInstances`] into the render
/// world. Gated on `Changed<>` in the main world, same as
/// `buildings::render::draw::extract_building_instances` - `AutomobilesInstances` is only ever
/// inserted once per tile (all motion happens in the vertex shader afterward), so this fires
/// exactly once per tile's lifetime rather than re-cloning every tile's instances every frame.
pub fn extract_automobiles_instances(
    mut commands: Commands,
    query: Extract<Query<(RenderEntity, &AutomobilesInstances), Changed<AutomobilesInstances>>>,
) {
    for (entity, instances) in &query {
        commands.entity(entity).insert(instances.clone());
    }
}

/// A single, persistent, reused GPU buffer holding every currently-active tile's automobile
/// instances, merged and baked to absolute (camera-relative) positions each frame - copy of
/// `buildings::render::draw::MergedBuildingInstances`.
#[derive(Resource)]
pub struct MergedAutomobilesInstances(pub RawBufferVec<InstanceData>);

impl Default for MergedAutomobilesInstances {
    fn default() -> Self {
        Self(RawBufferVec::new(BufferUsages::VERTEX))
    }
}

/// A persistent placeholder entity used as the `Transparent3d` phase item's associated entity,
/// since automobiles are rendered via one merged draw call rather than per-tile entities - copy of
/// `buildings::render::draw::BuildingsRendererEntity`.
#[derive(Resource)]
pub struct AutomobilesRendererEntity(pub Entity);

impl FromWorld for AutomobilesRendererEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(Name::new("AutomobilesRenderer")).id())
    }
}

pub fn prepare_merged_automobiles_buffer(
    mut merged: ResMut<MergedAutomobilesInstances>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    tiles: Query<(&AutomobilesInstances, &AutomobilesTileOrigin)>,
) {
    merged.0.values_mut().clear();

    for (instances, origin) in &tiles {
        for instance in &instances.0 {
            let mut baked = *instance;
            for waypoint in &mut baked.waypoints {
                waypoint[0] += origin.translation.x;
                waypoint[1] += origin.translation.y;
                waypoint[2] += origin.translation.z;
            }
            merged.0.push(baked);
        }
    }

    merged.0.write_buffer(&render_device, &render_queue);
}

pub struct DrawAutomobilesInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawAutomobilesInstanced {
    type Param = (SRes<TruckMeshBuffer>, SRes<MergedAutomobilesInstances>);
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (truck, merged): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let merged = merged.into_inner();
        let Some(buffer) = merged.0.buffer() else {
            return RenderCommandResult::Skip;
        };
        let truck = truck.into_inner();

        pass.set_vertex_buffer(0, truck.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, buffer.slice(..));
        pass.set_index_buffer(truck.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..truck.index_count, 0, 0..merged.0.len() as u32);

        RenderCommandResult::Success
    }
}

pub type DrawAutomobiles = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetRenderParamsBindGroup<1>,
    DrawAutomobilesInstanced,
);

pub fn queue_automobiles(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    automobiles_pipeline: Res<AutomobilesPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<AutomobilesPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_key_cache: Res<ViewKeyCache>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    merged: Res<MergedAutomobilesInstances>,
    renderer_entity: Res<AutomobilesRendererEntity>,
) {
    if merged.0.is_empty() {
        return;
    }

    let draw_function = draw_functions.read().get_id::<DrawAutomobiles>().unwrap();
    let entity = renderer_entity.0;
    let main_entity = MainEntity::from(entity);

    for view in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &automobiles_pipeline,
            AutomobilesPipelineKey { view_key },
        );

        transparent_phase.add_transient(Transparent3d {
            sorting_info: TransparentSortingInfo3d::AlwaysOnTop,
            distance: 0.0,
            entity: (entity, main_entity),
            draw_function,
            pipeline,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}
