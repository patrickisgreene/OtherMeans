use bevy::{
    material::descriptor::{BindGroupLayoutDescriptor, ComputePipelineDescriptor},
    prelude::*,
    render::{
        render_resource::{AsBindGroup, SpecializedComputePipeline},
        renderer::RenderDevice,
    },
};

use crate::{
    render::{
        IndirectBindGroup, PrepassViewBindGroup, TerrainBindGroup, TerrainViewBindGroup,
        TilingPrepassPipelineKey,
    },
    shaders::{PREPARE_PREPASS_SHADER, REFINE_TILES_SHADER},
};

#[derive(Resource)]
pub struct TerrainTilingPrepassPipelines {
    pub(crate) terrain_layout: BindGroupLayoutDescriptor,
    pub(crate) terrain_view_layout: BindGroupLayoutDescriptor,
    pub(crate) indirect_layout: BindGroupLayoutDescriptor,
    pub(crate) prepass_view_layout: BindGroupLayoutDescriptor,
    prepare_prepass_shader: Handle<Shader>,
    refine_tiles_shader: Handle<Shader>,
}

impl FromWorld for TerrainTilingPrepassPipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let terrain_layout = TerrainBindGroup::bind_group_layout_descriptor(device);
        let terrain_view_layout = TerrainViewBindGroup::bind_group_layout_descriptor(device);
        let indirect_layout = IndirectBindGroup::bind_group_layout_descriptor(device);
        let prepass_view_layout = PrepassViewBindGroup::bind_group_layout_descriptor(device);

        let prepare_prepass_shader = world.load_asset(PREPARE_PREPASS_SHADER);
        let refine_tiles_shader = world.load_asset(REFINE_TILES_SHADER);

        TerrainTilingPrepassPipelines {
            terrain_view_layout,
            indirect_layout,
            prepass_view_layout,
            terrain_layout,
            prepare_prepass_shader,
            refine_tiles_shader,
        }
    }
}

impl SpecializedComputePipeline for TerrainTilingPrepassPipelines {
    type Key = TilingPrepassPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut layout = default();
        let mut shader = default();
        let mut entry_point = default();

        let shader_defs = key.shader_defs();

        if key.contains(TilingPrepassPipelineKey::REFINE_TILES) {
            layout = vec![
                self.prepass_view_layout.clone(),
                self.terrain_layout.clone(),
            ];
            shader = self.refine_tiles_shader.clone();
            entry_point = Some("refine_tiles".into());
        }
        if key.contains(TilingPrepassPipelineKey::PREPARE_ROOT) {
            layout = vec![
                self.prepass_view_layout.clone(),
                self.terrain_layout.clone(),
                self.indirect_layout.clone(),
            ];
            shader = self.prepare_prepass_shader.clone();
            entry_point = Some("prepare_root".into());
        }
        if key.contains(TilingPrepassPipelineKey::PREPARE_NEXT) {
            layout = vec![
                self.prepass_view_layout.clone(),
                self.terrain_layout.clone(),
                self.indirect_layout.clone(),
            ];
            shader = self.prepare_prepass_shader.clone();
            entry_point = Some("prepare_next".into());
        }
        if key.contains(TilingPrepassPipelineKey::PREPARE_RENDER) {
            layout = vec![
                self.prepass_view_layout.clone(),
                self.terrain_layout.clone(),
                self.indirect_layout.clone(),
            ];
            shader = self.prepare_prepass_shader.clone();
            entry_point = Some("prepare_render".into());
        }

        ComputePipelineDescriptor {
            label: Some("tiling_prepass_pipeline".into()),
            layout,
            immediate_size: Default::default(),
            shader,
            shader_defs,
            entry_point,
            zero_initialize_workgroup_memory: false,
        }
    }
}
