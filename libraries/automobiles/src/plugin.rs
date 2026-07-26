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

use crate::instances::{AutomobilesInstances, update_automobiles_batches};
use crate::render::{
    draw::{
        AutomobilesRendererEntity, DrawAutomobiles, MergedAutomobilesInstances,
        extract_automobiles_instances, prepare_merged_automobiles_buffer, queue_automobiles,
    },
    origin::AutomobilesTileOrigin,
    pipeline::{
        AutomobilesPipeline, init_automobiles_pipeline, init_render_params_bind_group_layout,
    },
    time::{
        AutomobilesRenderParams, RenderParamsBuffer, prepare_render_params_bind_group,
        prepare_render_params_buffer, update_automobiles_render_params,
    },
};

/// Renders small instanced boxes animating along roads as ambient, purely-visual traffic.
///
/// Requires [`buildings::BuildingsPlugin`] (for `buildings::tile_height`, used to place automobiles
/// at real terrain elevation) and [`roads::RoadsPlugin`] (for the `RoadNetwork` asset loader) to
/// already be added to the app.
pub struct AutomobilesPlugin;

impl Plugin for AutomobilesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutomobilesRenderParams>()
            .add_systems(
                Update,
                (update_automobiles_batches, update_automobiles_render_params),
            )
            .add_plugins((
                SyncComponentPlugin::<AutomobilesInstances>::default(),
                ExtractComponentPlugin::<AutomobilesTileOrigin>::default(),
                ExtractResourcePlugin::<AutomobilesRenderParams>::default(),
            ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<bevy::core_pipeline::core_3d::Transparent3d, DrawAutomobiles>()
            .init_resource::<SpecializedRenderPipelines<AutomobilesPipeline>>()
            .init_resource::<MergedAutomobilesInstances>()
            .init_resource::<AutomobilesRendererEntity>()
            .init_resource::<RenderParamsBuffer>()
            .add_systems(
                RenderStartup,
                (
                    init_render_params_bind_group_layout,
                    init_automobiles_pipeline.after(MeshPipelineSystems),
                )
                    .chain(),
            )
            .add_systems(ExtractSchedule, extract_automobiles_instances)
            .add_systems(
                Render,
                (
                    prepare_merged_automobiles_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_render_params_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    queue_automobiles.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
