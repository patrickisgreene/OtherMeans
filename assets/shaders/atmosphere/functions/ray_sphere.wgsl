#import "shaders/atmosphere/functions/helpers.wgsl"::sub_split_vec3

// ============================================================================
// Ray-sphere intersection
// ============================================================================

// Returns (distance_to_near, distance_to_far) or (-1, -1) if no hit
fn ray_sphere(ray_origin: vec3<f32>, ray_dir: vec3<f32>, sphere_center: vec3<f32>, radius: f32) -> vec2<f32> {
    let oc = ray_origin - sphere_center;
    let b = dot(oc, ray_dir);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = b * b - c;

    if discriminant < 0.0 {
        return vec2(-1.0, -1.0);
    }

    let sqrt_disc = sqrt(discriminant);
    let t0 = -b - sqrt_disc;
    let t1 = -b + sqrt_disc;

    return vec2(t0, t1);
}

// High-precision version using split coordinates
fn ray_sphere_precise(
    center_high: vec3<f32>, center_low: vec3<f32>,
    radius: f32,
    origin_high: vec3<f32>, origin_low: vec3<f32>,
    ray_dir: vec3<f32>
) -> vec2<f32> {
    let oc = sub_split_vec3(origin_high, origin_low, center_high, center_low);
    let b = dot(oc, ray_dir);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = b * b - c;

    if discriminant < 0.0 {
        return vec2(-1.0, -1.0);
    }

    let sqrt_disc = sqrt(discriminant);
    let t0 = -b - sqrt_disc;
    let t1 = -b + sqrt_disc;

    return vec2(t0, t1);
}
