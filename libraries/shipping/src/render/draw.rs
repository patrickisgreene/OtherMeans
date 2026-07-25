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

use crate::instances::{ShippingInstances, InstanceData};
use crate::render::origin::ShippingTileOrigin;
use crate::render::pipeline::{ShipMeshBuffer, ShippingPipeline, ShippingPipelineKey};
use crate::render::time::SetRenderParamsBindGroup;

/// Extracts newly-spawned (or, in principle, changed) [`VehicleInstances`] into the render
/// world. Gated on `Changed<>` in the main world, same as
/// `buildings::render::draw::extract_building_instances` - `VehicleInstances` is only ever
/// inserted once per tile (all motion happens in the vertex shader afterward), so this fires
/// exactly once per tile's lifetime rather than re-cloning every tile's instances every frame.
pub fn extract_shipping_instances(
    mut commands: Commands,
    query: Extract<Query<(RenderEntity, &ShippingInstances), Changed<ShippingInstances>>>,
) {
    for (entity, instances) in &query {
        commands.entity(entity).insert(instances.clone());
    }
}

/// A single, persistent, reused GPU buffer holding every currently-active tile's vehicle
/// instances, merged and baked to absolute (camera-relative) positions each frame - copy of
/// `buildings::render::draw::MergedBuildingInstances`.
#[derive(Resource)]
pub struct MergedShippingInstances(pub RawBufferVec<InstanceData>);

impl Default for MergedShippingInstances {
    fn default() -> Self {
        Self(RawBufferVec::new(BufferUsages::VERTEX))
    }
}

/// A persistent placeholder entity used as the `Transparent3d` phase item's associated entity,
/// since vehicles are rendered via one merged draw call rather than per-tile entities - copy of
/// `buildings::render::draw::BuildingsRendererEntity`.
#[derive(Resource)]
pub struct ShippingRendererEntity(pub Entity);

impl FromWorld for ShippingRendererEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(Name::new("VehiclesRenderer")).id())
    }
}

pub fn prepare_merged_shipping_buffer(
    mut merged: ResMut<MergedShippingInstances>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    tiles: Query<(&ShippingInstances, &ShippingTileOrigin)>,
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

pub struct DrawShippingInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawShippingInstanced {
    type Param = (SRes<ShipMeshBuffer>, SRes<MergedShippingInstances>);
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (ship, merged): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let merged = merged.into_inner();
        let Some(buffer) = merged.0.buffer() else {
            return RenderCommandResult::Skip;
        };
        let ship = ship.into_inner();

        pass.set_vertex_buffer(0, ship.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, buffer.slice(..));
        pass.set_index_buffer(ship.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..ship.index_count, 0, 0..merged.0.len() as u32);

        RenderCommandResult::Success
    }
}

pub type DrawShipping = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetRenderParamsBindGroup<1>,
    DrawShippingInstanced,
);

pub fn queue_shipping(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    shipping_pipeline: Res<ShippingPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ShippingPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    view_key_cache: Res<ViewKeyCache>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    merged: Res<MergedShippingInstances>,
    renderer_entity: Res<ShippingRendererEntity>,
) {
    if merged.0.is_empty() {
        return;
    }

    let draw_function = draw_functions.read().get_id::<DrawShipping>().unwrap();
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
            &shipping_pipeline,
            ShippingPipelineKey { view_key },
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
