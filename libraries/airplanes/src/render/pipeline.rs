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
use crate::render::time::AirplaneRenderParams;

pub const AIRPLANE_SHADER: &str = "shaders/airplanes.wgsl";

/// A low-poly airliner silhouette: a fuselage tapering to a point at the nose, a wide flat wing
/// crossing it near the middle, and a small vertical tail fin near the rear. Local axes match the
/// old unit-cube convention (X = width/wingspan, Y = height, Z = length: -Z = tail, +Z = nose,
/// each roughly -0.5..0.5) so the existing per-instance `dimensions`/`color_and_width.w` scaling
/// keeps working unchanged, exactly like `shipping::render::pipeline::ShipVertex`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct AirplaneVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

// Fuselage: a narrow box tapering to a single point at the nose.
const FUSE_HALF_WIDTH: f32 = 0.06;
const FUSE_HALF_HEIGHT: f32 = 0.05;
const TAIL_Z: f32 = -0.5;
const NOSE_Z: f32 = 0.5;

// Wing: a wide, thin box reaching full wingspan (matches `color_and_width.w`'s scale).
const WING_HALF_SPAN: f32 = 0.5;
const WING_HALF_THICK: f32 = 0.015;
const WING_BACK_Z: f32 = -0.05;
const WING_FRONT_Z: f32 = 0.12;

// Tail fin: a small vertical box near the rear, mounted on top of the fuselage.
const FIN_HALF_X: f32 = 0.02;
const FIN_BACK_Z: f32 = -0.5;
const FIN_FRONT_Z: f32 = -0.32;
const FIN_TOP_Y: f32 = 0.22;

/// The 21 unique corners, indexed by `AIRPLANE_INDICES` below: 0-4 fuselage, 5-12 wing, 13-20 tail
/// fin.
fn airplane_positions() -> [Vec3; 21] {
    [
        // Fuselage: tail rectangle (0-3) + nose apex (4).
        Vec3::new(-FUSE_HALF_WIDTH, -FUSE_HALF_HEIGHT, TAIL_Z), // 0: tail-left-bottom
        Vec3::new(FUSE_HALF_WIDTH, -FUSE_HALF_HEIGHT, TAIL_Z),  // 1: tail-right-bottom
        Vec3::new(FUSE_HALF_WIDTH, FUSE_HALF_HEIGHT, TAIL_Z),   // 2: tail-right-top
        Vec3::new(-FUSE_HALF_WIDTH, FUSE_HALF_HEIGHT, TAIL_Z),  // 3: tail-left-top
        Vec3::new(0.0, 0.0, NOSE_Z),                            // 4: nose apex
        // Wing.
        Vec3::new(-WING_HALF_SPAN, -WING_HALF_THICK, WING_BACK_Z), // 5: back-left-bottom
        Vec3::new(WING_HALF_SPAN, -WING_HALF_THICK, WING_BACK_Z),  // 6: back-right-bottom
        Vec3::new(-WING_HALF_SPAN, -WING_HALF_THICK, WING_FRONT_Z), // 7: front-left-bottom
        Vec3::new(WING_HALF_SPAN, -WING_HALF_THICK, WING_FRONT_Z), // 8: front-right-bottom
        Vec3::new(-WING_HALF_SPAN, WING_HALF_THICK, WING_BACK_Z),  // 9: back-left-top
        Vec3::new(WING_HALF_SPAN, WING_HALF_THICK, WING_BACK_Z),   // 10: back-right-top
        Vec3::new(-WING_HALF_SPAN, WING_HALF_THICK, WING_FRONT_Z), // 11: front-left-top
        Vec3::new(WING_HALF_SPAN, WING_HALF_THICK, WING_FRONT_Z),  // 12: front-right-top
        // Tail fin.
        Vec3::new(-FIN_HALF_X, FUSE_HALF_HEIGHT, FIN_BACK_Z), // 13: back-left-bottom
        Vec3::new(FIN_HALF_X, FUSE_HALF_HEIGHT, FIN_BACK_Z),  // 14: back-right-bottom
        Vec3::new(-FIN_HALF_X, FUSE_HALF_HEIGHT, FIN_FRONT_Z), // 15: front-left-bottom
        Vec3::new(FIN_HALF_X, FUSE_HALF_HEIGHT, FIN_FRONT_Z),  // 16: front-right-bottom
        Vec3::new(-FIN_HALF_X, FIN_TOP_Y, FIN_BACK_Z),        // 17: back-left-top
        Vec3::new(FIN_HALF_X, FIN_TOP_Y, FIN_BACK_Z),         // 18: back-right-top
        Vec3::new(-FIN_HALF_X, FIN_TOP_Y, FIN_FRONT_Z),       // 19: front-left-top
        Vec3::new(FIN_HALF_X, FIN_TOP_Y, FIN_FRONT_Z),        // 20: front-right-top
    ]
}

/// Triangle indices, wound counter-clockwise when viewed from outside each face (matching
/// `PrimitiveState { cull_mode: Some(Face::Back), .. }` below): the fuselage's flat tail cap and
/// its four nose-taper sides, the wing's full six-face box (it floats clear of the fuselage, so
/// unlike `shipping`'s cargo/bridge boxes it needs a bottom face too), and the tail fin's box
/// (bottom face omitted - it sits flush on the fuselage, never visible from outside).
const AIRPLANE_INDICES: [u16; 84] = [
    // Fuselage: tail cap, then the four nose-taper sides.
    0, 3, 2, 0, 2, 1, //
    0, 1, 4, //
    1, 2, 4, //
    2, 3, 4, //
    3, 0, 4, //
    // Wing: back, front, left, right, top, bottom.
    5, 9, 10, 5, 10, 6, //
    7, 8, 12, 7, 12, 11, //
    5, 7, 11, 5, 11, 9, //
    6, 10, 12, 6, 12, 8, //
    9, 11, 12, 9, 12, 10, //
    5, 8, 7, 5, 6, 8, //
    // Tail fin: back, front, left, right, top.
    13, 17, 18, 13, 18, 14, //
    15, 16, 20, 15, 20, 19, //
    13, 15, 19, 13, 19, 17, //
    14, 18, 20, 14, 20, 16, //
    17, 19, 20, 17, 20, 18, //
];

/// Builds the airplane's vertex/index buffers, computing each vertex's normal as the normalized
/// average of its adjacent faces' normals - copy of `shipping::render::pipeline::ship_mesh`.
fn airplane_mesh() -> (Vec<AirplaneVertex>, Vec<u16>) {
    let positions = airplane_positions();
    let mut normals = [Vec3::ZERO; 21];

    for triangle in AIRPLANE_INDICES.chunks_exact(3) {
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
        .map(|(&position, &normal)| AirplaneVertex {
            position: position.to_array(),
            normal: normal.normalize().to_array(),
        })
        .collect();

    (vertices, AIRPLANE_INDICES.to_vec())
}

/// A resource holding the shared airplane vertex/index buffers used by every plane instance
/// batch.
#[derive(Resource)]
pub struct AirplaneMeshBuffer {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

/// The bind group layout for the per-frame [`AirplaneRenderParams`] uniform (group 1).
#[derive(Resource)]
pub struct RenderParamsBindGroupLayout {
    pub layout: BindGroupLayoutDescriptor,
}

pub fn init_render_params_bind_group_layout(mut commands: Commands) {
    let layout = BindGroupLayoutDescriptor::new(
        "AirplaneRenderParams layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX,
            uniform_buffer::<AirplaneRenderParams>(false),
        ),
    );

    commands.insert_resource(RenderParamsBindGroupLayout { layout });
}

/// The pipeline used to render instanced, animated airplane boxes.
#[derive(Resource)]
pub struct AirplanePipeline {
    mesh_pipeline: MeshPipeline,
    render_params_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

pub fn init_airplane_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    render_params_layout: Res<RenderParamsBindGroupLayout>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    commands.insert_resource(AirplanePipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        render_params_layout: render_params_layout.layout.clone(),
        shader: asset_server.load(AIRPLANE_SHADER),
    });

    let (vertices, indices) = airplane_mesh();
    let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("airplanes_airplane_vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });
    let index_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("airplanes_airplane_index_buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: BufferUsages::INDEX,
    });

    commands.insert_resource(AirplaneMeshBuffer {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
    });
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AirplanePipelineKey {
    pub view_key: MeshPipelineKey,
}

fn airplane_vertex_buffer_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: size_of::<AirplaneVertex>() as u64,
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

/// One `Float32x4` attribute per `InstanceData` field, starting at location 2 (after the mesh's
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

impl SpecializedRenderPipeline for AirplanePipeline {
    type Key = AirplanePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let view_layout = self.mesh_pipeline.get_view_layout(key.view_key.into());

        RenderPipelineDescriptor {
            label: Some("airplanes_pipeline".into()),
            layout: vec![
                view_layout.main_layout.clone(),
                self.render_params_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                shader_defs: vec![],
                buffers: vec![airplane_vertex_buffer_layout(), instance_buffer_layout()],
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure-math check, independent of the whole render pipeline (which this sandbox can't
    /// reliably run to get a screenshot from): every triangle's winding, combined with
    /// `PrimitiveState { cull_mode: Some(Face::Back), .. }`, must produce a normal pointing away
    /// from the mesh's own center. If a triangle's winding is backwards, it gets culled from
    /// every outside viewing angle - a mesh where *every* face is backwards would render as
    /// completely invisible regardless of instance size or position, exactly matching the
    /// reported symptom, so this is worth ruling in or out directly.
    /// Which sub-part (fuselage, wing, or fin) each vertex index belongs to - the mesh is a union
    /// of three separate convex-ish pieces (see `airplane_positions`' doc comment), so "outward"
    /// has to be judged relative to each piece's own center, not one shared centroid for the
    /// whole (very non-convex) mesh.
    fn part_of(index: u16) -> std::ops::Range<u16> {
        match index {
            0..=4 => 0..5,
            5..=12 => 5..13,
            _ => 13..21,
        }
    }

    #[test]
    fn every_triangle_normal_points_outward() {
        let positions = airplane_positions();
        let part_center = |range: std::ops::Range<u16>| -> Vec3 {
            let slice = &positions[range.start as usize..range.end as usize];
            slice.iter().copied().sum::<Vec3>() / slice.len() as f32
        };

        let mut inward = Vec::new();
        for (triangle_index, triangle) in AIRPLANE_INDICES.chunks_exact(3).enumerate() {
            let a = positions[triangle[0] as usize];
            let b = positions[triangle[1] as usize];
            let c = positions[triangle[2] as usize];

            let normal = (b - a).cross(c - a);
            assert!(
                normal.length() > 1e-8,
                "triangle {triangle_index} ({triangle:?}) is degenerate (zero area)"
            );

            let center = part_center(part_of(triangle[0]));
            let centroid = (a + b + c) / 3.0;
            let outward = centroid - center;
            let alignment = normal.normalize().dot(outward.normalize());
            if alignment <= 0.0 {
                inward.push((triangle_index, triangle.to_vec(), alignment));
            }
        }

        assert!(
            inward.is_empty(),
            "found {} triangle(s) winding inward (would be back-face culled from outside): {inward:?}",
            inward.len()
        );
    }

    /// Every index must reference one of the mesh's actual unique vertex positions.
    #[test]
    fn every_index_is_in_range() {
        let positions = airplane_positions();
        for &index in AIRPLANE_INDICES.iter() {
            assert!(
                (index as usize) < positions.len(),
                "index {index} is out of range for {} positions",
                positions.len()
            );
        }
    }
}
