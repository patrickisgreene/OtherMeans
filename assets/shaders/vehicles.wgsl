#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::position_world_to_clip

struct RenderParams {
    elapsed_secs: f32,
    blend_distance: f32,
    blend_range: f32,
    max_lod: f32,
}

@group(1) @binding(0) var<uniform> render_params: RenderParams;

const WAYPOINT_COUNT: u32 = 8u;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) waypoint0: vec4<f32>,
    @location(3) waypoint1: vec4<f32>,
    @location(4) waypoint2: vec4<f32>,
    @location(5) waypoint3: vec4<f32>,
    @location(6) waypoint4: vec4<f32>,
    @location(7) waypoint5: vec4<f32>,
    @location(8) waypoint6: vec4<f32>,
    @location(9) waypoint7: vec4<f32>,
    // x = waypoint count, y = speed (m/s), z = phase (metres travelled at t=0).
    @location(10) path_params: vec4<f32>,
    @location(11) instance_normal: vec4<f32>,
    // rgb = color, a = footprint width.
    @location(12) color_and_width: vec4<f32>,
    // x = length, y = height.
    @location(13) dimensions: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
}

// Mirrors `buildings`' `compute_fade` (see libraries/buildings/src/shaders/buildings.wgsl) -
// vehicles live at the same fixed highest LOD as buildings, so they fade out over the same
// terrain LOD blend window instead of popping when their tile leaves the active set.
fn compute_fade(world_position: vec3<f32>) -> f32 {
    let view_distance = distance(world_position, view.world_position);
    let blend_range = max(render_params.blend_range, 0.0001);
    let target_lod = log2(render_params.blend_distance / max(view_distance, 0.0001));
    return saturate((target_lod - (render_params.max_lod - blend_range)) / blend_range);
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var waypoints: array<vec4<f32>, WAYPOINT_COUNT>;
    waypoints[0] = input.waypoint0;
    waypoints[1] = input.waypoint1;
    waypoints[2] = input.waypoint2;
    waypoints[3] = input.waypoint3;
    waypoints[4] = input.waypoint4;
    waypoints[5] = input.waypoint5;
    waypoints[6] = input.waypoint6;
    waypoints[7] = input.waypoint7;

    let count = max(u32(input.path_params.x), 1u);
    let speed = input.path_params.y;
    let phase = input.path_params.z;
    let direction = input.path_params.w;

    let total_length = max(waypoints[count - 1u].w, 0.0001);
    let raw_dist = speed * render_params.elapsed_secs + phase;
    let dist_forward = raw_dist - floor(raw_dist / total_length) * total_length;
    // Reverse-direction vehicles walk the same waypoint path back-to-front, so they visually
    // drive the opposite way down the road instead of just facing backward.
    let dist = select(dist_forward, total_length - dist_forward, direction < 0.0);

    var segment_start = waypoints[0];
    var segment_end = waypoints[0];
    for (var i: u32 = 0u; i < WAYPOINT_COUNT - 1u; i = i + 1u) {
        if (i >= count - 1u) {
            break;
        }
        segment_start = waypoints[i];
        segment_end = waypoints[i + 1u];
        if (dist <= segment_end.w) {
            break;
        }
    }

    let segment_length = max(segment_end.w - segment_start.w, 0.0001);
    let t = clamp((dist - segment_start.w) / segment_length, 0.0, 1.0);
    let path_position = mix(segment_start.xyz, segment_end.xyz, t);

    let up = normalize(input.instance_normal.xyz);
    var forward_raw = segment_end.xyz - segment_start.xyz;
    if (dot(forward_raw, forward_raw) < 1e-6) {
        forward_raw = vec3<f32>(1.0, 0.0, 0.0);
    }
    forward_raw = forward_raw - up * dot(forward_raw, up);
    if (dot(forward_raw, forward_raw) < 1e-6) {
        forward_raw = vec3<f32>(0.0, 0.0, 1.0) - up * dot(vec3<f32>(0.0, 0.0, 1.0), up);
    }
    let right = normalize(cross(up, forward_raw));
    let forward = normalize(cross(right, up));

    let scaled = input.position * vec3<f32>(input.color_and_width.w, input.dimensions.y, input.dimensions.x);
    let offset = right * scaled.x + up * scaled.y + forward * scaled.z;
    // Mirrors buildings' `+ normal * (height * 0.5)` (see
    // `libraries/buildings/src/instances.rs::generate_tile_instances`) - `path_position` is
    // exactly at ground elevation, so without this lift the box would be centered on the ground
    // point (half embedded below the visible surface) instead of resting on top of it.
    let world_position = path_position + up * (input.dimensions.y * 0.5) + offset;

    var output: VertexOutput;
    output.clip_position = position_world_to_clip(world_position);
    output.world_normal = right * input.normal.x + up * input.normal.y + forward * input.normal.z;
    output.color = vec4<f32>(input.color_and_width.rgb, compute_fade(world_position));
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.5));
    let diffuse = 0.5 + 0.5 * max(dot(normalize(input.world_normal), light_dir), 0.0);
    return vec4<f32>(input.color.rgb * diffuse, input.color.a);
}
