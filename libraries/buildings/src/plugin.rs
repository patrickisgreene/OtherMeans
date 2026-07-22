use bevy::{
    asset::embedded_asset,
    pbr::MeshPipelineSystems,
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        render_phase::AddRenderCommand,
        render_resource::SpecializedRenderPipelines,
        sync_component::SyncComponentPlugin,
        ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    },
};

use crate::height::{HeightMap, HeightMapLoader};
use crate::instances::{update_building_batches, BuildingInstances};
use crate::ocean_mask::{OceanMask, OceanMaskLoader};
use crate::population::{PopulationDensity, PopulationDensityLoader};
use crate::render::{
    draw::{
        extract_building_instances, prepare_merged_building_buffer, queue_buildings,
        BuildingsRendererEntity, DrawBuildings, MergedBuildingInstances,
    },
    origin::BuildingTileOrigin,
    pipeline::{init_buildings_pipeline, BuildingsPipeline},
};

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/buildings.wgsl");

        app.init_asset::<PopulationDensity>()
            .register_asset_loader(PopulationDensityLoader)
            .init_asset::<OceanMask>()
            .register_asset_loader(OceanMaskLoader)
            .init_asset::<HeightMap>()
            .register_asset_loader(HeightMapLoader)
            .add_systems(Update, update_building_batches)
            .add_plugins((
                SyncComponentPlugin::<BuildingInstances>::default(),
                ExtractComponentPlugin::<BuildingTileOrigin>::default(),
            ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<bevy::core_pipeline::core_3d::Transparent3d, DrawBuildings>()
            .init_resource::<SpecializedRenderPipelines<BuildingsPipeline>>()
            .init_resource::<MergedBuildingInstances>()
            .init_resource::<BuildingsRendererEntity>()
            .add_systems(RenderStartup, init_buildings_pipeline.after(MeshPipelineSystems))
            .add_systems(ExtractSchedule, extract_building_instances)
            .add_systems(
                Render,
                (
                    prepare_merged_building_buffer.in_set(RenderSystems::PrepareResources),
                    queue_buildings.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
