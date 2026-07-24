use crate::{
    config::TerrainConfig,
    data::{AttachmentLabel, TileAtlas, TileTree, tile_atlas::gpu::GpuTileAtlas},
    formats::TiffLoader,
    mipmap::{MipPipelines, mip_prepass},
    render::{
        TerrainItem, TerrainTilingPrepassPipelines, pass::systems::extract_terrain_phases,
        prepare_terrain_depth_textures, terrain_pass, tiling_prepass,
    },
    shaders::{InternalShaders, load_terrain_shaders},
};
use bevy::{
    core_pipeline::{Core3dSystems, core_3d::main_opaque_pass_3d, schedule::Core3d},
    prelude::*,
    ecs::schedule::ApplyDeferred,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_phase::{DrawFunctions, ViewSortedRenderPhases, sort_phase_system},
        render_resource::*,
        renderer::{RenderGraph, RenderGraphSystems},
        sync_component::SyncComponentPlugin,
    },
};
use bevy_common_assets::ron::RonAssetPlugin;

#[derive(Resource)]
pub struct TerrainSettings {
    pub attachments: Vec<AttachmentLabel>,
    pub atlas_size: u32,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            attachments: vec![AttachmentLabel::Height],
            atlas_size: 1028,
        }
    }
}

impl TerrainSettings {
    pub fn new(custom_attachments: Vec<&str>) -> Self {
        let mut attachments = vec![AttachmentLabel::Height];
        attachments.extend(
            custom_attachments
                .into_iter()
                .map(|name| AttachmentLabel::Custom(name.into())),
        );

        Self {
            attachments,
            atlas_size: 1028,
        }
    }
}

/// The plugin for the terrain renderer.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<TerrainConfig>::new(&["terrain.ron"]),
            SyncComponentPlugin::<TileTree>::default(),
            SyncComponentPlugin::<TileAtlas>::default(),
        ))
            .init_asset::<TerrainConfig>()
            .init_resource::<InternalShaders>()
            .init_resource::<TerrainSettings>()
            .init_asset_loader::<TiffLoader>()
            .add_systems(
                PostUpdate,
                (
                    crate::data::tile_tree::systems::compute_requests,
                    crate::data::tile_loader::systems::finish_loading,
                    crate::data::tile_atlas::systems::update,
                    crate::data::tile_loader::systems::start_loading,
                    crate::data::tile_tree::systems::adjust_to_tile_atlas,
                    crate::data::tile_tree::systems::generate_surface_approximation,
                    crate::data::tile_tree::systems::update_terrain_view_buffer,
                    crate::data::tile_atlas::systems::update_terrain_buffer,
                )
                    .chain()
                    .after(TransformSystems::Propagate),
            );
        app.sub_app_mut(RenderApp)
            .init_resource::<SpecializedComputePipelines<MipPipelines>>()
            .init_resource::<SpecializedComputePipelines<TerrainTilingPrepassPipelines>>()
            .init_resource::<DrawFunctions<TerrainItem>>()
            .init_resource::<ViewSortedRenderPhases<TerrainItem>>()
            .add_systems(
                RenderStartup,
                (
                    crate::mipmap::init_mip_pipelines,
                    crate::render::pipelines::init_tiling_prepass_pipelines,
                    crate::render::pass::init_depth_copy_pipeline,
                ),
            )
            .add_systems(
                ExtractSchedule,
                (
                    extract_terrain_phases,
                    (
                        GpuTileAtlas::initialize,
                        ApplyDeferred,
                        (
                            GpuTileAtlas::extract,
                            crate::render::bind_group::systems::initialize,
                        ),
                    )
                        .chain(),
                    crate::render::view_bind_group::systems::initialize,
                ),
            )
            .add_systems(
                Render,
                (
                    (
                        GpuTileAtlas::prepare,
                        crate::render::bind_group::systems::prepare,
                        crate::render::view_bind_group::systems::prepare_terrain_view,
                        crate::render::view_bind_group::systems::prepare_indirect,
                        crate::render::view_bind_group::systems::prepare_refine_tiles,
                    )
                        .in_set(RenderSystems::Prepare),
                    sort_phase_system::<TerrainItem>.in_set(RenderSystems::PhaseSort),
                    prepare_terrain_depth_textures.in_set(RenderSystems::PrepareResources),
                    (
                        tiling_prepass::systems::queue_tiling_prepass,
                        GpuTileAtlas::queue,
                    )
                        .in_set(RenderSystems::Queue),
                    GpuTileAtlas::_cleanup
                        .before(World::clear_entities)
                        .in_set(RenderSystems::Cleanup),
                ),
            )
            .add_systems(
                Core3d,
                terrain_pass
                    .before(main_opaque_pass_3d)
                    .in_set(Core3dSystems::MainPass),
            )
            .add_systems(
                RenderGraph,
                (mip_prepass, tiling_prepass)
                    .chain()
                    .in_set(RenderGraphSystems::Begin),
            );
    }

    fn finish(&self, app: &mut App) {
        let attachments = app
            .world()
            .resource::<TerrainSettings>()
            .attachments
            .clone();

        load_terrain_shaders(app, &attachments);
    }
}
