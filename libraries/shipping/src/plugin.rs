use bevy::{
    pbr::MeshPipelineSystems,
    prelude::*,
    render::{
        ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::ExtractComponentPlugin, extract_resource::ExtractResourcePlugin,
        render_phase::AddRenderCommand, render_resource::SpecializedRenderPipelines,
        sync_component::SyncComponentPlugin,
    },
};

use crate::instances::{ShippingInstances, update_shipping_batches};
use crate::render::{
    draw::{
        ShippingRendererEntity, DrawShipping, MergedShippingInstances, extract_shipping_instances,
        prepare_merged_shipping_buffer, queue_shipping,
    },
    origin::ShippingTileOrigin,
    pipeline::{ShippingPipeline, init_shipping_pipeline, init_render_params_bind_group_layout},
    time::{
        ShippingRenderParams, RenderParamsBuffer, prepare_render_params_bind_group,
        prepare_render_params_buffer, update_shipping_render_params,
    },
};

/// Renders small instanced boxes animating along roads as ambient, purely-visual traffic.
///
/// Requires [`buildings::BuildingsPlugin`] (for `buildings::tile_height`, used to place vehicles
/// at real terrain elevation) and [`roads::RoadsPlugin`] (for the `RoadNetwork` asset loader) to
/// already be added to the app.
pub struct ShippingPlugin;

impl Plugin for ShippingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShippingRenderParams>()
            .add_systems(Update, (update_shipping_batches, update_shipping_render_params))
            .add_plugins((
                SyncComponentPlugin::<ShippingInstances>::default(),
                ExtractComponentPlugin::<ShippingTileOrigin>::default(),
                ExtractResourcePlugin::<ShippingRenderParams>::default(),
            ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<bevy::core_pipeline::core_3d::Transparent3d, DrawShipping>()
            .init_resource::<SpecializedRenderPipelines<ShippingPipeline>>()
            .init_resource::<MergedShippingInstances>()
            .init_resource::<ShippingRendererEntity>()
            .init_resource::<RenderParamsBuffer>()
            .add_systems(
                RenderStartup,
                (
                    init_render_params_bind_group_layout,
                    init_shipping_pipeline.after(MeshPipelineSystems),
                )
                    .chain(),
            )
            .add_systems(ExtractSchedule, extract_shipping_instances)
            .add_systems(
                Render,
                (
                    prepare_merged_shipping_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    queue_shipping.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
