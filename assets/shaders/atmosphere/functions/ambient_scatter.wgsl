#import "shaders/atmosphere/functions/atmosphere.wgsl"::atmosphere_density
#import "shaders/atmosphere/consts.wgsl"::{NUM_SAMPLES, AMBIENT_SCATTER_COLOR}
#import "shaders/atmosphere/functions/ray_sphere.wgsl"::ray_sphere
#import "shaders/atmosphere/bindings.wgsl"::settings

// Calculate ambient scattering for night side visibility
// This is a simplified calculation that just considers the ray path through atmosphere
fn calculate_ambient_scatter(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    max_dist: f32,
    planet_radius: f32,
    atmo_radius: f32,
) -> vec3<f32> {
    let atmo_hit = ray_sphere(ray_origin, ray_dir, vec3(0.0), atmo_radius);

    if atmo_hit.y < 0.0 {
        return vec3(0.0);
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
        return vec3(0.0);
    }

    // Accumulate density along ray
    let segment_length = (t_end - t_start) / f32(NUM_SAMPLES);
    var total_density = 0.0;

    for (var i = 0; i < NUM_SAMPLES; i++) {
        let t = t_start + (f32(i) + 0.5) * segment_length;
        let sample_pos = ray_origin + ray_dir * t;
        let height = length(sample_pos) - planet_radius;
        let height_normalized = height / (atmo_radius - planet_radius);

        if height_normalized >= 0.0 && height_normalized <= 1.0 {
            let density = atmosphere_density(height_normalized);
            total_density += density.x * segment_length;  // Use Rayleigh density
        }
    }

    // Return ambient color scaled by density
    return AMBIENT_SCATTER_COLOR * total_density * settings.ambient_scatter_strength;
}
