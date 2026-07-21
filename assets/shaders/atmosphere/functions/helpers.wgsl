#import "shaders/atmosphere/bindings.wgsl"::{view, settings}

// ============================================================================
// 64-bit precision helpers
// ============================================================================

fn sub_split_vec3(
    a_high: vec3<f32>, a_low: vec3<f32>,
    b_high: vec3<f32>, b_low: vec3<f32>
) -> vec3<f32> {
    return (a_high - b_high) + (a_low - b_low);
}

// ============================================================================
// Coordinate helpers
// ============================================================================

fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2., -2.) + vec2(-1., 1.);
}

fn frag_coord_to_uv(frag_coord: vec2<f32>) -> vec2<f32> {
    return (frag_coord - view.viewport.xy) / view.viewport.zw;
}

fn frag_coord_to_ndc(frag_coord: vec4<f32>) -> vec3<f32> {
    return vec3(uv_to_ndc(frag_coord_to_uv(frag_coord.xy)), frag_coord.z);
}

fn get_ray_direction(ndc: vec2<f32>) -> vec3<f32> {
    // Transform from clip space to view space
    let clip_pos = vec4<f32>(ndc, 1.0, 1.0);
    let view_pos = settings.inverse_proj * clip_pos;

    // Get direction in view space (perspective divide and normalize)
    let view_dir = normalize(view_pos.xyz / view_pos.w);

    // Transform to world space using only the rotation part of inverse_view
    // Extract the 3x3 rotation matrix from inverse_view
    let world_dir = (settings.inverse_view * vec4<f32>(view_dir, 0.0)).xyz;

    return normalize(world_dir);
}
