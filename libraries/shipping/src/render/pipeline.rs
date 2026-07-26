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
use crate::render::time::ShippingRenderParams;

pub const SHIPPING_SHADER: &str = "shaders/shipping.wgsl";

/// A low-poly container-ship silhouette: a hull that tapers to a pointed bow, a low flat deck, a
/// block amidships standing in for stacked containers, and a smaller, taller bridge tower set
/// back near the stern - the reverse of a truck's layout (cab low and at the front, trailer tall
/// and at the back), which is what actually reads as "ship" rather than "truck" at a glance.
/// Local axes match the old unit-cube convention (X = width, Y = height, Z = length: -Z = stern,
/// +Z = bow, each roughly -0.5..0.5) so the existing per-instance `dimensions`/`color_and_width.w`
/// scaling and the ground-placement lift (`shaders/shipping.wgsl`'s `up * (height * 0.5)`) keep
/// working unchanged. The hull bottom sits slightly below local Y = -0.5 so the lift (which
/// places local Y = 0 at sea level) leaves a bit of draft below the waterline instead of the hull
/// floating entirely on top of it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ShipVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

/// Keel line - slightly below the local-space origin (sea level after the shader's lift) for a
/// bit of visible draft.
const HULL_BOTTOM_Y: f32 = -0.55;
/// Main deck height - kept low, since most of a container ship's visual bulk is the stacked
/// cargo/bridge sitting on top of it rather than hull freeboard.
const DECK_Y: f32 = -0.35;
/// Z at which the hull sides start tapering in toward the bow point.
const BOW_TAPER_Z: f32 = 0.25;
/// Bow tip.
const BOW_Z: f32 = 0.5;
/// Stern.
const STERN_Z: f32 = -0.5;

/// Cargo block (stacked containers) footprint/height.
const CARGO_X: f32 = 0.42;
const CARGO_BACK_Z: f32 = -0.30;
const CARGO_FRONT_Z: f32 = 0.15;
const CARGO_TOP_Y: f32 = 0.05;

/// Bridge/superstructure tower, set back near the stern like a real container ship.
const BRIDGE_X: f32 = 0.22;
const BRIDGE_BACK_Z: f32 = -0.48;
const BRIDGE_FRONT_Z: f32 = -0.32;
const BRIDGE_TOP_Y: f32 = 0.30;

/// The 26 unique corners, indexed by `SHIP_INDICES` below: 0-9 hull, 10-17 cargo block, 18-25
/// bridge.
fn ship_positions() -> [Vec3; 26] {
    [
        // Hull.
        Vec3::new(-0.5, HULL_BOTTOM_Y, STERN_Z), // 0: stern-left-bottom
        Vec3::new(0.5, HULL_BOTTOM_Y, STERN_Z),  // 1: stern-right-bottom
        Vec3::new(-0.5, HULL_BOTTOM_Y, BOW_TAPER_Z), // 2: mid-left-bottom
        Vec3::new(0.5, HULL_BOTTOM_Y, BOW_TAPER_Z), // 3: mid-right-bottom
        Vec3::new(0.0, HULL_BOTTOM_Y, BOW_Z),    // 4: bow-bottom
        Vec3::new(-0.5, DECK_Y, STERN_Z),        // 5: stern-left-deck
        Vec3::new(0.5, DECK_Y, STERN_Z),         // 6: stern-right-deck
        Vec3::new(-0.5, DECK_Y, BOW_TAPER_Z),    // 7: mid-left-deck
        Vec3::new(0.5, DECK_Y, BOW_TAPER_Z),     // 8: mid-right-deck
        Vec3::new(0.0, DECK_Y, BOW_Z),           // 9: bow-deck
        // Cargo block.
        Vec3::new(-CARGO_X, DECK_Y, CARGO_BACK_Z), // 10: back-left-bottom
        Vec3::new(CARGO_X, DECK_Y, CARGO_BACK_Z),  // 11: back-right-bottom
        Vec3::new(-CARGO_X, DECK_Y, CARGO_FRONT_Z), // 12: front-left-bottom
        Vec3::new(CARGO_X, DECK_Y, CARGO_FRONT_Z), // 13: front-right-bottom
        Vec3::new(-CARGO_X, CARGO_TOP_Y, CARGO_BACK_Z), // 14: back-left-top
        Vec3::new(CARGO_X, CARGO_TOP_Y, CARGO_BACK_Z), // 15: back-right-top
        Vec3::new(-CARGO_X, CARGO_TOP_Y, CARGO_FRONT_Z), // 16: front-left-top
        Vec3::new(CARGO_X, CARGO_TOP_Y, CARGO_FRONT_Z), // 17: front-right-top
        // Bridge.
        Vec3::new(-BRIDGE_X, DECK_Y, BRIDGE_BACK_Z), // 18: back-left-bottom
        Vec3::new(BRIDGE_X, DECK_Y, BRIDGE_BACK_Z),  // 19: back-right-bottom
        Vec3::new(-BRIDGE_X, DECK_Y, BRIDGE_FRONT_Z), // 20: front-left-bottom
        Vec3::new(BRIDGE_X, DECK_Y, BRIDGE_FRONT_Z), // 21: front-right-bottom
        Vec3::new(-BRIDGE_X, BRIDGE_TOP_Y, BRIDGE_BACK_Z), // 22: back-left-top
        Vec3::new(BRIDGE_X, BRIDGE_TOP_Y, BRIDGE_BACK_Z), // 23: back-right-top
        Vec3::new(-BRIDGE_X, BRIDGE_TOP_Y, BRIDGE_FRONT_Z), // 24: front-left-top
        Vec3::new(BRIDGE_X, BRIDGE_TOP_Y, BRIDGE_FRONT_Z), // 25: front-right-top
    ]
}

/// Triangle indices, wound counter-clockwise when viewed from outside each face (matching
/// `PrimitiveState { cull_mode: Some(Face::Back), .. }` below): the hull's parallel midbody, its
/// tapered bow wedge, the cargo block, and the bridge tower (both boxes omit their bottom face -
/// they sit flush on the deck, which is never visible from below).
const SHIP_INDICES: [u16; 108] = [
    // Hull midbody: bottom, deck, stern transom, left, right.
    0, 1, 3, 0, 3, 2, //
    5, 7, 8, 5, 8, 6, //
    0, 5, 6, 0, 6, 1, //
    0, 2, 7, 0, 7, 5, //
    1, 6, 8, 1, 8, 3, //
    // Hull bow taper: bottom, deck, left, right.
    2, 3, 4, //
    7, 9, 8, //
    2, 4, 9, 2, 9, 7, //
    3, 8, 9, 3, 9, 4, //
    // Cargo block: back, front, left, right, top.
    10, 14, 15, 10, 15, 11, //
    12, 13, 17, 12, 17, 16, //
    10, 12, 16, 10, 16, 14, //
    11, 15, 17, 11, 17, 13, //
    14, 16, 17, 14, 17, 15, //
    // Bridge: back, front, left, right, top.
    18, 22, 23, 18, 23, 19, //
    20, 21, 25, 20, 25, 24, //
    18, 20, 24, 18, 24, 22, //
    19, 23, 25, 19, 25, 21, //
    22, 24, 25, 22, 25, 23, //
];

/// Builds the ship's vertex/index buffers, computing each vertex's normal as the normalized
/// average of its adjacent faces' normals - since vertices are shared across faces (the whole
/// point of indexing down to unique positions), there's no per-face-duplicated vertex to give a
/// crisp flat normal to, so this reads as gently smooth-shaded rather than hard-edged.
fn ship_mesh() -> (Vec<ShipVertex>, Vec<u16>) {
    let positions = ship_positions();
    let mut normals = [Vec3::ZERO; 26];

    for triangle in SHIP_INDICES.chunks_exact(3) {
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
        .map(|(&position, &normal)| ShipVertex {
            position: position.to_array(),
            normal: normal.normalize().to_array(),
        })
        .collect();

    (vertices, SHIP_INDICES.to_vec())
}

/// A resource holding the shared ship vertex/index buffers used by every shipping instance batch.
#[derive(Resource)]
pub struct ShipMeshBuffer {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

/// The bind group layout for the per-frame [`ShippingRenderParams`] uniform (group 1).
#[derive(Resource)]
pub struct RenderParamsBindGroupLayout {
    pub layout: BindGroupLayoutDescriptor,
}

pub fn init_render_params_bind_group_layout(mut commands: Commands) {
    let layout = BindGroupLayoutDescriptor::new(
        "ShippingsRenderParams layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX,
            uniform_buffer::<ShippingRenderParams>(false),
        ),
    );

    commands.insert_resource(RenderParamsBindGroupLayout { layout });
}

/// The pipeline used to render instanced, animated boat boxes.
#[derive(Resource)]
pub struct ShippingPipeline {
    mesh_pipeline: MeshPipeline,
    render_params_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

pub fn init_shipping_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    render_params_layout: Res<RenderParamsBindGroupLayout>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    commands.insert_resource(ShippingPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        render_params_layout: render_params_layout.layout.clone(),
        shader: asset_server.load(SHIPPING_SHADER),
    });

    let (vertices, indices) = ship_mesh();
    let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("shipping_ship_vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });
    let index_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("shipping_ship_index_buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: BufferUsages::INDEX,
    });

    commands.insert_resource(ShipMeshBuffer {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
    });
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ShippingPipelineKey {
    pub view_key: MeshPipelineKey,
}

fn ship_vertex_buffer_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: size_of::<ShipVertex>() as u64,
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

impl SpecializedRenderPipeline for ShippingPipeline {
    type Key = ShippingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let view_layout = self.mesh_pipeline.get_view_layout(key.view_key.into());

        RenderPipelineDescriptor {
            label: Some("shipping_pipeline".into()),
            layout: vec![
                view_layout.main_layout.clone(),
                self.render_params_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                shader_defs: vec![],
                buffers: vec![ship_vertex_buffer_layout(), instance_buffer_layout()],
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
