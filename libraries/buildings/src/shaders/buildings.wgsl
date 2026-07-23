#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::position_world_to_clip

struct FadeParams {
    blend_distance: f32,
    blend_range: f32,
    max_lod: f32,
}

@group(1) @binding(0) var<uniform> fade_params: FadeParams;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) instance_position_footprint: vec4<f32>,
    @location(3) instance_rotation: vec4<f32>,
    @location(4) instance_color_height: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
}

fn rotate_by_quat(v: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

// Mirrors terrain's own `compute_blend` (see libraries/terrain/src/shaders/functions.wgsl) -
// `target_lod` is a continuous "which LOD does this distance want" curve, equal to `max_lod`
// exactly at the finest LOD's ideal distance. Buildings only ever live at `max_lod`, so unlike
// terrain (which blends across every LOD boundary) they only care about this one boundary:
// fully visible while `target_lod >= max_lod`, fading to invisible over a `blend_range`-wide
// window as it drops below that.
fn compute_fade(world_position: vec3<f32>) -> f32 {
    let view_distance = distance(world_position, view.world_position);
    let blend_range = max(fade_params.blend_range, 0.0001);
    let target_lod = log2(fade_params.blend_distance / max(view_distance, 0.0001));
    return saturate((target_lod - (fade_params.max_lod - blend_range)) / blend_range);
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let footprint = input.instance_position_footprint.w;
    let height = input.instance_color_height.a;
    let local_position = input.position * vec3<f32>(footprint, height, footprint);
    let rotated_position = rotate_by_quat(local_position, input.instance_rotation);
    let world_position = input.instance_position_footprint.xyz + rotated_position;

    var output: VertexOutput;
    output.clip_position = position_world_to_clip(world_position);
    output.world_normal = rotate_by_quat(input.normal, input.instance_rotation);
    output.color = vec4<f32>(input.instance_color_height.rgb, compute_fade(world_position));
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.5));
    let diffuse = 0.5 + 0.5 * max(dot(normalize(input.world_normal), light_dir), 0.0);
    return vec4<f32>(input.color.rgb * diffuse, input.color.a);
}
