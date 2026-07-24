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

use crate::instances::{InstanceData, VehicleInstances};
use crate::render::origin::VehicleTileOrigin;
use crate::render::pipeline::{TruckMeshBuffer, VehiclesPipeline, VehiclesPipelineKey};
use crate::render::time::SetRenderParamsBindGroup;

/// Extracts newly-spawned (or, in principle, changed) [`VehicleInstances`] into the render
/// world. Gated on `Changed<>` in the main world, same as
/// `buildings::render::draw::extract_building_instances` - `VehicleInstances` is only ever
/// inserted once per tile (all motion happens in the vertex shader afterward), so this fires
/// exactly once per tile's lifetime rather than re-cloning every tile's instances every frame.
pub fn extract_vehicle_instances(
    mut commands: Commands,
    query: Extract<Query<(RenderEntity, &VehicleInstances), Changed<VehicleInstances>>>,
) {
    for (entity, instances) in &query {
        commands.entity(entity).insert(instances.clone());
    }
}

/// A single, persistent, reused GPU buffer holding every currently-active tile's vehicle
/// instances, merged and baked to absolute (camera-relative) positions each frame - copy of
/// `buildings::render::draw::MergedBuildingInstances`.
#[derive(Resource)]
pub struct MergedVehicleInstances(pub RawBufferVec<InstanceData>);

impl Default for MergedVehicleInstances {
    fn default() -> Self {
        Self(RawBufferVec::new(BufferUsages::VERTEX))
    }
}

/// A persistent placeholder entity used as the `Transparent3d` phase item's associated entity,
/// since vehicles are rendered via one merged draw call rather than per-tile entities - copy of
/// `buildings::render::draw::BuildingsRendererEntity`.
#[derive(Resource)]
pub struct VehiclesRendererEntity(pub Entity);

impl FromWorld for VehiclesRendererEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(Name::new("VehiclesRenderer")).id())
    }
}

pub fn prepare_merged_vehicle_buffer(
    mut merged: ResMut<MergedVehicleInstances>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    tiles: Query<(&VehicleInstances, &VehicleTileOrigin)>,
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

pub struct DrawVehiclesInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawVehiclesInstanced {
    type Param = (SRes<TruckMeshBuffer>, SRes<MergedVehicleInstances>);
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

pub type DrawVehicles = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetRenderParamsBindGroup<1>,
    DrawVehiclesInstanced,
);

pub fn queue_vehicles(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    vehicles_pipeline: Res<VehiclesPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<VehiclesPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_key_cache: Res<ViewKeyCache>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    merged: Res<MergedVehicleInstances>,
    renderer_entity: Res<VehiclesRendererEntity>,
) {
    if merged.0.is_empty() {
        return;
    }

    let draw_function = draw_functions.read().get_id::<DrawVehicles>().unwrap();
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
            &vehicles_pipeline,
            VehiclesPipelineKey { view_key },
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
