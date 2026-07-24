use bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy::{
    asset::{AssetServer, Handle},
    mesh::VertexBufferLayout,
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        render_resource::{binding_types::uniform_buffer, *},
        renderer::RenderDevice,
    },
    shader::Shader,
};

use crate::instances::InstanceData;
use crate::render::fade::BuildingsFadeParams;

pub const BUILDINGS_SHADER: &str = "shaders/buildings.wgsl";

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CubeVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

fn cube_face(normal: Vec3, right: Vec3) -> [CubeVertex; 6] {
    let up = normal.cross(right);
    let corner = |right_sign: f32, up_sign: f32| -> Vec3 {
        0.5 * normal + right_sign * 0.5 * right + up_sign * 0.5 * up
    };
    let vertex = |position: Vec3| CubeVertex {
        position: position.to_array(),
        normal: normal.to_array(),
    };

    let c00 = corner(-1.0, -1.0);
    let c10 = corner(1.0, -1.0);
    let c11 = corner(1.0, 1.0);
    let c01 = corner(-1.0, 1.0);

    [
        vertex(c00),
        vertex(c10),
        vertex(c11),
        vertex(c00),
        vertex(c11),
        vertex(c01),
    ]
}

fn cube_vertices() -> Vec<CubeVertex> {
    [
        cube_face(Vec3::X, Vec3::Y),
        cube_face(Vec3::NEG_X, Vec3::Y),
        cube_face(Vec3::Y, Vec3::X),
        cube_face(Vec3::NEG_Y, Vec3::X),
        cube_face(Vec3::Z, Vec3::X),
        cube_face(Vec3::NEG_Z, Vec3::X),
    ]
    .concat()
}

/// A resource holding the shared unit-cube vertex buffer used by every building instance batch.
#[derive(Resource)]
pub struct CubeVertexBuffer {
    pub buffer: Buffer,
    pub vertex_count: u32,
}

/// The bind group layout for the per-frame [`BuildingsFadeParams`] uniform (group 1).
#[derive(Resource)]
pub struct FadeParamsBindGroupLayout {
    pub layout: BindGroupLayoutDescriptor,
}

pub fn init_fade_params_bind_group_layout(mut commands: Commands) {
    let layout = BindGroupLayoutDescriptor::new(
        "BuildingsFadeParams layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX,
            uniform_buffer::<BuildingsFadeParams>(false),
        ),
    );

    commands.insert_resource(FadeParamsBindGroupLayout { layout });
}

/// The pipeline used to render instanced building cubes.
#[derive(Resource)]
pub struct BuildingsPipeline {
    mesh_pipeline: MeshPipeline,
    fade_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

pub fn init_buildings_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    fade_layout: Res<FadeParamsBindGroupLayout>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    commands.insert_resource(BuildingsPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        fade_layout: fade_layout.layout.clone(),
        shader: asset_server.load(BUILDINGS_SHADER),
    });

    let vertices = cube_vertices();
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("building_cube_vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });

    commands.insert_resource(CubeVertexBuffer {
        buffer,
        vertex_count: vertices.len() as u32,
    });
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BuildingsPipelineKey {
    pub view_key: MeshPipelineKey,
}

fn cube_vertex_buffer_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: size_of::<CubeVertex>() as u64,
        step_mode: VertexStepMode::Vertex,
        attributes: vec![
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
        ],
    }
}

fn instance_buffer_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: size_of::<InstanceData>() as u64,
        step_mode: VertexStepMode::Instance,
        attributes: vec![
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 16,
                shader_location: 3,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 32,
                shader_location: 4,
            },
        ],
    }
}

impl SpecializedRenderPipeline for BuildingsPipeline {
    type Key = BuildingsPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let view_layout = self.mesh_pipeline.get_view_layout(key.view_key.into());

        RenderPipelineDescriptor {
            label: Some("buildings_pipeline".into()),
            layout: vec![view_layout.main_layout.clone(), self.fade_layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                shader_defs: vec![],
                buffers: vec![cube_vertex_buffer_layout(), instance_buffer_layout()],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some("fragment".into()),
                shader_defs: vec![],
                targets: vec![Some(ColorTargetState {
                    format: key.view_key.target_format(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Greater),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.view_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            ..default()
        }
    }
}
