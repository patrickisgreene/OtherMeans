use std::marker::PhantomData;

use bevy::{
    material::descriptor::{
        BindGroupLayoutDescriptor, FragmentState, RenderPipelineDescriptor, VertexState,
    },
    mesh::PrimitiveTopology,
    pbr::{MeshPipeline, MeshPipelineViewLayoutKey},
    prelude::*,
    render::{
        render_resource::{
            BlendState, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, Face, FrontFace, MultisampleState, PrimitiveState,
            SpecializedRenderPipeline, StencilFaceState, StencilOperation, StencilState,
            TextureFormat,
        },
        renderer::RenderDevice,
    },
    shader::ShaderRef,
};

use crate::{
    render::{TERRAIN_DEPTH_FORMAT, TerrainTilingPrepassPipelines, material::TerrainPipelineKey},
    shaders::{DEFAULT_FRAGMENT_SHADER, DEFAULT_VERTEX_SHADER},
};

/// The pipeline used to render the terrain entities.
#[derive(Resource)]
pub struct TerrainRenderPipeline<M: Material> {
    mesh_pipeline: MeshPipeline,
    terrain_layout: BindGroupLayoutDescriptor,
    terrain_view_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    marker: PhantomData<M>,
}

pub fn init_terrain_render_pipeline<M: Material>(
    mut commands: Commands,
    device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
    prepass_pipelines: Res<TerrainTilingPrepassPipelines>,
) {
    let vertex_shader = match M::vertex_shader() {
        ShaderRef::Default => asset_server.load(DEFAULT_VERTEX_SHADER),
        ShaderRef::Handle(handle) => handle,
        ShaderRef::Path(path) => asset_server.load(path),
    };

    let fragment_shader = match M::fragment_shader() {
        ShaderRef::Default => asset_server.load(DEFAULT_FRAGMENT_SHADER),
        ShaderRef::Handle(handle) => handle,
        ShaderRef::Path(path) => asset_server.load(path),
    };

    commands.insert_resource(TerrainRenderPipeline::<M> {
        mesh_pipeline: mesh_pipeline.clone(),
        terrain_layout: prepass_pipelines.terrain_layout.clone(),
        terrain_view_layout: prepass_pipelines.terrain_view_layout.clone(),
        material_layout: M::bind_group_layout_descriptor(&device),
        vertex_shader,
        fragment_shader,
        marker: PhantomData,
    });
}

impl<M: Material> SpecializedRenderPipeline for TerrainRenderPipeline<M> {
    type Key = TerrainPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = key.flags.shader_defs();

        if key
            .view_layout_key
            .contains(MeshPipelineViewLayoutKey::MULTISAMPLED)
        {
            shader_defs.push("MULTISAMPLED".into());
        }

        let mut bind_group_layout = vec![
            self.mesh_pipeline
                .get_view_layout(key.view_layout_key)
                .main_layout
                .clone(),
        ];

        bind_group_layout.push(self.terrain_layout.clone());
        bind_group_layout.push(self.terrain_view_layout.clone());
        bind_group_layout.push(self.material_layout.clone());

        let mut vertex_shader_defs = shader_defs.clone();
        vertex_shader_defs.push("VERTEX".into());
        let mut fragment_shader_defs = shader_defs.clone();
        fragment_shader_defs.push("FRAGMENT".into());

        RenderPipelineDescriptor {
            label: None,
            layout: bind_group_layout,
            immediate_size: Default::default(),
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                entry_point: Some("vertex".into()),
                shader_defs: vertex_shader_defs,
                buffers: Vec::new(),
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: key.flags.polygon_mode(),
                conservative: false,
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: fragment_shader_defs,
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TERRAIN_DEPTH_FORMAT,
                depth_write_enabled: true.into(),
                depth_compare: CompareFunction::Greater.into(),
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::GreaterEqual,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                        pass_op: StencilOperation::Replace,
                    },
                    back: StencilFaceState::IGNORE,
                    read_mask: !0,
                    write_mask: !0,
                },
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.flags.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            zero_initialize_workgroup_memory: false,
        }
    }
}
