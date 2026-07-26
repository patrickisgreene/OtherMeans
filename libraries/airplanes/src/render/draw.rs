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

use crate::instances::{AirplaneInstances, InstanceData};
use crate::render::origin::AirplaneTileOrigin;
use crate::render::pipeline::{AirplaneMeshBuffer, AirplanePipeline, AirplanePipelineKey};
use crate::render::time::SetRenderParamsBindGroup;

/// Extracts newly-spawned (or, in principle, changed) [`AirplaneInstances`] into the render
/// world. Gated on `Changed<>` in the main world - `AirplaneInstances` is only ever inserted once
/// per tile (all motion happens in the vertex shader afterward), so this fires exactly once per
/// tile's lifetime rather than re-cloning every tile's instances every frame. Copy of
/// `shipping::render::draw::extract_shipping_instances`.
pub fn extract_airplane_instances(
    mut commands: Commands,
    query: Extract<Query<(RenderEntity, &AirplaneInstances), Changed<AirplaneInstances>>>,
) {
    for (entity, instances) in &query {
        commands.entity(entity).insert(instances.clone());
    }
}

/// A single, persistent, reused GPU buffer holding every currently-active tile's plane instances,
/// merged and baked to absolute (camera-relative) positions each frame - copy of
/// `shipping::render::draw::MergedShippingInstances`.
#[derive(Resource)]
pub struct MergedAirplaneInstances(pub RawBufferVec<InstanceData>);

impl Default for MergedAirplaneInstances {
    fn default() -> Self {
        Self(RawBufferVec::new(BufferUsages::VERTEX))
    }
}

/// A persistent placeholder entity used as the `Transparent3d` phase item's associated entity,
/// since planes are rendered via one merged draw call rather than per-tile entities - copy of
/// `shipping::render::draw::ShippingRendererEntity`.
#[derive(Resource)]
pub struct AirplaneRendererEntity(pub Entity);

impl FromWorld for AirplaneRendererEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(Name::new("AirplanesRenderer")).id())
    }
}

pub fn prepare_merged_airplane_buffer(
    mut merged: ResMut<MergedAirplaneInstances>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    tiles: Query<(&AirplaneInstances, &AirplaneTileOrigin)>,
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

pub struct DrawAirplanesInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawAirplanesInstanced {
    type Param = (SRes<AirplaneMeshBuffer>, SRes<MergedAirplaneInstances>);
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (airplane, merged): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let merged = merged.into_inner();
        let Some(buffer) = merged.0.buffer() else {
            return RenderCommandResult::Skip;
        };
        let airplane = airplane.into_inner();

        pass.set_vertex_buffer(0, airplane.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, buffer.slice(..));
        pass.set_index_buffer(airplane.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..airplane.index_count, 0, 0..merged.0.len() as u32);

        RenderCommandResult::Success
    }
}

pub type DrawAirplanes = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetRenderParamsBindGroup<1>,
    DrawAirplanesInstanced,
);

pub fn queue_airplanes(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    airplane_pipeline: Res<AirplanePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<AirplanePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_key_cache: Res<ViewKeyCache>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    merged: Res<MergedAirplaneInstances>,
    renderer_entity: Res<AirplaneRendererEntity>,
) {
    if merged.0.is_empty() {
        return;
    }

    let draw_function = draw_functions.read().get_id::<DrawAirplanes>().unwrap();
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
            &airplane_pipeline,
            AirplanePipelineKey { view_key },
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
