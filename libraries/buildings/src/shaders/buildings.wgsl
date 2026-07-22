#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::position_world_to_clip

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
    output.color = vec4<f32>(input.instance_color_height.rgb, 1.0);
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.5));
    let diffuse = 0.5 + 0.5 * max(dot(normalize(input.world_normal), light_dir), 0.0);
    return vec4<f32>(input.color.rgb * diffuse, 1.0);
}
