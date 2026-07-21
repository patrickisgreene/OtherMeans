#import "shaders/atmosphere/consts.wgsl"::{RAYLEIGH_COEFF, MIE_COEFF, NUM_SAMPLES}
#import "shaders/atmosphere/functions/atmosphere.wgsl"::atmosphere_density
#import "shaders/atmosphere/functions/ray_sphere.wgsl"::ray_sphere

// Calculate transmittance through atmosphere for original scene color
fn calculate_transmittance(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    max_dist: f32,
    planet_radius: f32,
    atmo_radius: f32,
) -> vec3<f32> {
    let atmo_hit = ray_sphere(ray_origin, ray_dir, vec3(0.0), atmo_radius);

    if atmo_hit.y < 0.0 {
        return vec3(1.0);
    }

    let t_start = max(0.0, atmo_hit.x);
    var t_end = atmo_hit.y;

    if max_dist > 0.0 {
        t_end = min(t_end, max_dist);
    }

    let planet_hit = ray_sphere(ray_origin, ray_dir, vec3(0.0), planet_radius);
    if planet_hit.x > 0.0 {
        t_end = min(t_end, planet_hit.x);
    }

    if t_end <= t_start {
        return vec3(1.0);
    }

    let segment_length = (t_end - t_start) / f32(NUM_SAMPLES);
    var optical_depth_r = 0.0;
    var optical_depth_m = 0.0;

    for (var i = 0; i < NUM_SAMPLES; i++) {
        let t = t_start + (f32(i) + 0.5) * segment_length;
        let sample_pos = ray_origin + ray_dir * t;
        let height = length(sample_pos) - planet_radius;
        let height_normalized = height / (atmo_radius - planet_radius);

        if height_normalized >= 0.0 && height_normalized <= 1.0 {
            let density = atmosphere_density(height_normalized);
            optical_depth_r += density.x * segment_length;
            optical_depth_m += density.y * segment_length;
        }
    }

    let tau = RAYLEIGH_COEFF * optical_depth_r + vec3(MIE_COEFF) * optical_depth_m;
    return exp(-tau);
}
