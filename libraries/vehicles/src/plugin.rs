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

use crate::instances::{VehicleInstances, update_vehicle_batches};
use crate::render::{
    draw::{
        DrawVehicles, MergedVehicleInstances, VehiclesRendererEntity, extract_vehicle_instances,
        prepare_merged_vehicle_buffer, queue_vehicles,
    },
    origin::VehicleTileOrigin,
    pipeline::{VehiclesPipeline, init_render_params_bind_group_layout, init_vehicles_pipeline},
    time::{
        RenderParamsBuffer, VehiclesRenderParams, prepare_render_params_bind_group,
        prepare_render_params_buffer, update_vehicles_render_params,
    },
};

/// Renders small instanced boxes animating along roads as ambient, purely-visual traffic.
///
/// Requires [`buildings::BuildingsPlugin`] (for `buildings::tile_height`, used to place vehicles
/// at real terrain elevation) and [`roads::RoadsPlugin`] (for the `RoadNetwork` asset loader) to
/// already be added to the app.
pub struct VehiclesPlugin;

impl Plugin for VehiclesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehiclesRenderParams>()
            .add_systems(
                Update,
                (update_vehicle_batches, update_vehicles_render_params),
            )
            .add_plugins((
                SyncComponentPlugin::<VehicleInstances>::default(),
                ExtractComponentPlugin::<VehicleTileOrigin>::default(),
                ExtractResourcePlugin::<VehiclesRenderParams>::default(),
            ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<bevy::core_pipeline::core_3d::Transparent3d, DrawVehicles>()
            .init_resource::<SpecializedRenderPipelines<VehiclesPipeline>>()
            .init_resource::<MergedVehicleInstances>()
            .init_resource::<VehiclesRendererEntity>()
            .init_resource::<RenderParamsBuffer>()
            .add_systems(
                RenderStartup,
                (
                    init_render_params_bind_group_layout,
                    init_vehicles_pipeline.after(MeshPipelineSystems),
                )
                    .chain(),
            )
            .add_systems(ExtractSchedule, extract_vehicle_instances)
            .add_systems(
                Render,
                (
                    prepare_merged_vehicle_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    queue_vehicles.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
