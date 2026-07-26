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

use crate::instances::{AirplaneInstances, update_airplane_batches};
use crate::render::{
    draw::{
        AirplaneRendererEntity, DrawAirplanes, MergedAirplaneInstances, extract_airplane_instances,
        prepare_merged_airplane_buffer, queue_airplanes,
    },
    origin::AirplaneTileOrigin,
    pipeline::{AirplanePipeline, init_airplane_pipeline, init_render_params_bind_group_layout},
    time::{
        AirplaneRenderParams, RenderParamsBuffer, prepare_render_params_bind_group,
        prepare_render_params_buffer, update_airplane_render_params,
    },
};

/// Renders small instanced planes flying along hub-and-spoke city routes as ambient,
/// purely-visual air traffic.
///
/// Requires [`cities::CitiesPlugin`] (for the `CitiesDatabase` RON asset loader, used to derive
/// the route network) to already be added to the app.
pub struct AirplanesPlugin;

impl Plugin for AirplanesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AirplaneRenderParams>()
            .add_systems(Update, (update_airplane_batches, update_airplane_render_params))
            .add_plugins((
                SyncComponentPlugin::<AirplaneInstances>::default(),
                ExtractComponentPlugin::<AirplaneTileOrigin>::default(),
                ExtractResourcePlugin::<AirplaneRenderParams>::default(),
            ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<bevy::core_pipeline::core_3d::Transparent3d, DrawAirplanes>()
            .init_resource::<SpecializedRenderPipelines<AirplanePipeline>>()
            .init_resource::<MergedAirplaneInstances>()
            .init_resource::<AirplaneRendererEntity>()
            .init_resource::<RenderParamsBuffer>()
            .add_systems(
                RenderStartup,
                (
                    init_render_params_bind_group_layout,
                    init_airplane_pipeline.after(MeshPipelineSystems),
                )
                    .chain(),
            )
            .add_systems(ExtractSchedule, extract_airplane_instances)
            .add_systems(
                Render,
                (
                    prepare_merged_airplane_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    queue_airplanes.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
