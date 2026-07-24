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

use crate::instances::{BuildingInstances, update_building_batches};
use crate::ocean_mask::{OceanMask, OceanMaskLoader};
use crate::population::{PopulationDensity, PopulationDensityLoader};
use crate::render::{
    draw::{
        BuildingsRendererEntity, DrawBuildings, MergedBuildingInstances,
        extract_building_instances, prepare_merged_building_buffer, queue_buildings,
    },
    fade::{
        BuildingsFadeParams, FadeParamsBuffer, prepare_fade_params_bind_group,
        prepare_fade_params_buffer,
    },
    origin::BuildingTileOrigin,
    pipeline::{BuildingsPipeline, init_buildings_pipeline, init_fade_params_bind_group_layout},
};

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<PopulationDensity>()
            .register_asset_loader(PopulationDensityLoader)
            .init_asset::<OceanMask>()
            .register_asset_loader(OceanMaskLoader)
            .init_resource::<BuildingsFadeParams>()
            .add_systems(Update, update_building_batches)
            .add_plugins((
                SyncComponentPlugin::<BuildingInstances>::default(),
                ExtractComponentPlugin::<BuildingTileOrigin>::default(),
                ExtractResourcePlugin::<BuildingsFadeParams>::default(),
            ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<bevy::core_pipeline::core_3d::Transparent3d, DrawBuildings>()
            .init_resource::<SpecializedRenderPipelines<BuildingsPipeline>>()
            .init_resource::<MergedBuildingInstances>()
            .init_resource::<BuildingsRendererEntity>()
            .init_resource::<FadeParamsBuffer>()
            .add_systems(
                RenderStartup,
                (
                    init_fade_params_bind_group_layout,
                    init_buildings_pipeline.after(MeshPipelineSystems),
                )
                    .chain(),
            )
            .add_systems(ExtractSchedule, extract_building_instances)
            .add_systems(
                Render,
                (
                    prepare_merged_building_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_fade_params_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_fade_params_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    queue_buildings.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
