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
use crate::render::time::VehiclesRenderParams;

pub const VEHICLES_SHADER: &str = "shaders/vehicles.wgsl";

/// A low-poly semi-truck silhouette: a tall box at the back (trailer, roof at local Y = +0.5,
/// running flat from the back all the way to `TRAILER_FRONT_Z`) then a sloped section dropping
/// to a lower "cab" roof height only over the front portion, with the front face staying
/// vertical (directly above the bottom-front edge, forming an enclosed box front). Local axes
/// match the old unit-cube convention (X = width, Y = height, Z = length, each in roughly
/// -0.5..0.5) so the existing per-instance `dimensions`/`color_and_width.w` scaling in the
/// shader, and the ground-placement lift (`shaders/vehicles.wgsl`'s `up * (height * 0.5)`), keep
/// working unchanged.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TruckVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

/// Y of the front-top (cab roof) corners; the back-top (trailer roof) corners are at +0.5, the
/// floor at -0.5 - so the cab roof sits a little over halfway up the trailer's height.
const CAB_ROOF_Y: f32 = 0.05;

/// Z of the trailer roof's front edge - where the flat trailer top ends and the slope down to
/// the cab roof begins. The back (trailer) runs from -0.5 to this, the front (cab) from this to
/// +0.5, so the cab reads as the shorter of the two sections, like a real tractor-trailer rig.
const TRAILER_FRONT_Z: f32 = 0.15;

/// The 10 unique corners, indexed by `TRUCK_INDICES` below.
fn truck_positions() -> [Vec3; 10] {
    [
        Vec3::new(-0.5, -0.5, -0.5),           // 0: bottom-back-left
        Vec3::new(0.5, -0.5, -0.5),            // 1: bottom-back-right
        Vec3::new(-0.5, -0.5, 0.5),            // 2: bottom-front-left
        Vec3::new(0.5, -0.5, 0.5),             // 3: bottom-front-right
        Vec3::new(-0.5, 0.5, -0.5),            // 4: top-back-left (trailer roof, back edge)
        Vec3::new(0.5, 0.5, -0.5),             // 5: top-back-right
        Vec3::new(-0.5, 0.5, TRAILER_FRONT_Z), // 6: top-trailer-left (trailer roof, front edge)
        Vec3::new(0.5, 0.5, TRAILER_FRONT_Z),  // 7: top-trailer-right
        Vec3::new(-0.5, CAB_ROOF_Y, 0.5),      // 8: top-cab-left (cab roof)
        Vec3::new(0.5, CAB_ROOF_Y, 0.5),       // 9: top-cab-right
    ]
}

/// Triangle indices for the 7 faces (bottom, back, front, left, right, flat trailer roof, sloped
/// cab roof), wound counter-clockwise when viewed from outside each face, matching
/// `PrimitiveState { cull_mode: Some(Face::Back), .. }` below. Left/right are pentagons (5
/// corners each, fanned from the back-bottom corner) since the roof now breaks at
/// `TRAILER_FRONT_Z` instead of running in one straight slope.
const TRUCK_INDICES: [u16; 48] = [
    0, 1, 3, 0, 3, 2, // bottom
    0, 4, 5, 0, 5, 1, // back
    2, 3, 9, 2, 9, 8, // front (the enclosed cab face)
    0, 2, 8, 0, 8, 6, 0, 6, 4, // left
    1, 5, 7, 1, 7, 9, 1, 9, 3, // right
    4, 6, 7, 4, 7, 5, // roof: flat trailer section
    6, 8, 9, 6, 9, 7, // roof: sloped section down to the cab
];

/// Builds the truck's vertex/index buffers, computing each vertex's normal as the normalized
/// average of its adjacent faces' normals - since all 10 vertices are shared across faces (the
/// whole point of indexing down to unique positions), there's no per-face-duplicated vertex to
/// give a crisp flat normal to, so this reads as gently smooth-shaded rather than hard-edged.
fn truck_mesh() -> (Vec<TruckVertex>, Vec<u16>) {
    let positions = truck_positions();
    let mut normals = [Vec3::ZERO; 10];

    for triangle in TRUCK_INDICES.chunks_exact(3) {
        let (a, b, c) = (
            positions[triangle[0] as usize],
            positions[triangle[1] as usize],
            positions[triangle[2] as usize],
        );
        let face_normal = (b - a).cross(c - a).normalize();
        for &index in triangle {
            normals[index as usize] += face_normal;
        }
    }

    let vertices = positions
        .iter()
        .zip(normals.iter())
        .map(|(&position, &normal)| TruckVertex {
            position: position.to_array(),
            normal: normal.normalize().to_array(),
        })
        .collect();

    (vertices, TRUCK_INDICES.to_vec())
}

/// A resource holding the shared truck vertex/index buffers used by every vehicle instance batch.
#[derive(Resource)]
pub struct TruckMeshBuffer {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

/// The bind group layout for the per-frame [`VehiclesRenderParams`] uniform (group 1).
#[derive(Resource)]
pub struct RenderParamsBindGroupLayout {
    pub layout: BindGroupLayoutDescriptor,
}

pub fn init_render_params_bind_group_layout(mut commands: Commands) {
    let layout = BindGroupLayoutDescriptor::new(
        "VehiclesRenderParams layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX,
            uniform_buffer::<VehiclesRenderParams>(false),
        ),
    );

    commands.insert_resource(RenderParamsBindGroupLayout { layout });
}

/// The pipeline used to render instanced, animated vehicle boxes.
#[derive(Resource)]
pub struct VehiclesPipeline {
    mesh_pipeline: MeshPipeline,
    render_params_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

pub fn init_vehicles_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    render_params_layout: Res<RenderParamsBindGroupLayout>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    commands.insert_resource(VehiclesPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        render_params_layout: render_params_layout.layout.clone(),
        shader: asset_server.load(VEHICLES_SHADER),
    });

    let (vertices, indices) = truck_mesh();
    let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("vehicle_truck_vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });
    let index_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("vehicle_truck_index_buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: BufferUsages::INDEX,
    });

    commands.insert_resource(TruckMeshBuffer {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
    });
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VehiclesPipelineKey {
    pub view_key: MeshPipelineKey,
}

fn truck_vertex_buffer_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: size_of::<TruckVertex>() as u64,
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

/// One `Float32x4` attribute per `InstanceData` field, starting at location 2 (after the cube's
/// own position/normal at 0/1). WGSL vertex attributes can't be arrays, so `InstanceData`'s
/// fixed-size `waypoints` array becomes 8 separate locations (2..=9) that the shader recombines
/// into an `array<vec4<f32>, 8>` local variable.
fn instance_buffer_layout() -> VertexBufferLayout {
    let mut attributes = Vec::new();
    for (index, offset) in (0..crate::network::MAX_WAYPOINTS)
        .map(|i| i * 16)
        .enumerate()
    {
        attributes.push(VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: offset as u64,
            shader_location: 2 + index as u32,
        });
    }

    let waypoints_size = crate::network::MAX_WAYPOINTS * 16;
    for (index, name_offset) in [0usize, 16, 32, 48].into_iter().enumerate() {
        attributes.push(VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: (waypoints_size + name_offset) as u64,
            shader_location: 2 + crate::network::MAX_WAYPOINTS as u32 + index as u32,
        });
    }

    VertexBufferLayout {
        array_stride: size_of::<InstanceData>() as u64,
        step_mode: VertexStepMode::Instance,
        attributes,
    }
}

impl SpecializedRenderPipeline for VehiclesPipeline {
    type Key = VehiclesPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let view_layout = self.mesh_pipeline.get_view_layout(key.view_key.into());

        RenderPipelineDescriptor {
            label: Some("vehicles_pipeline".into()),
            layout: vec![
                view_layout.main_layout.clone(),
                self.render_params_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                shader_defs: vec![],
                buffers: vec![truck_vertex_buffer_layout(), instance_buffer_layout()],
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
