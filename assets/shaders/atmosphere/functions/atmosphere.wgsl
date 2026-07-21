#import "shaders/atmosphere/consts.wgsl"::{
    MIE_COEFF,
    NUM_SAMPLES, MIE_G, NUM_LIGHT_SAMPLES, MIN_HEIGHT,
    MIE_SCALE_HEIGHT, PI, RAYLEIGH_COEFF, RAYLEIGH_SCALE_HEIGHT
}
#import "shaders/atmosphere/functions/ray_sphere.wgsl"::ray_sphere
// ============================================================================
// Atmosphere density functions
// ============================================================================

// Returns density at a given height (0 = surface, 1 = top of atmosphere)
fn atmosphere_density(height_normalized: f32) -> vec2<f32> {
    let rayleigh = exp(-height_normalized / RAYLEIGH_SCALE_HEIGHT);
    let mie = exp(-height_normalized / MIE_SCALE_HEIGHT);
    return vec2(rayleigh, mie);
}

// Phase functions
fn rayleigh_phase(cos_theta: f32) -> f32 {
    return 3.0 / (16.0 * PI) * (1.0 + cos_theta * cos_theta);
}

fn mie_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let num = 3.0 * (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    let denom = (8.0 * PI) * (2.0 + g2) * pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5);
    return num / denom;
}

// ============================================================================
// Main atmosphere calculation
// ============================================================================

// psuedorandom helper
fn hash2to1(p: vec2<f32>) -> f32 {
    let p0 = fract(p * vec2<f32>(123.45, 456.78));
    let p1 = p0 + dot(p0, p0 + 45.67);
    return fract(p1.x * p1.y);
}

fn calculate_atmosphere(
    ray_origin: vec3<f32>,       // Camera position relative to planet center (normalized)
    ray_dir: vec3<f32>,          // Normalized ray direction
    sun_dir: vec3<f32>,          // Normalized direction to sun
    max_dist: f32,               // Maximum ray distance (normalized), -1 for infinity
    raw_depth: f32,
    planet_radius: f32,          // Normalized (typically 1.0)
    atmo_radius: f32,            // Normalized (typically 1.025)
) -> vec3<f32> {
    // Intersect atmosphere
    let atmo_hit = ray_sphere(ray_origin, ray_dir, vec3(0.0), atmo_radius);

    if atmo_hit.y < 0.0 {
        // No atmosphere intersection
        return vec3(0.0);
    }

    // Calculate ray segment through atmosphere
    let t_start = max(0.0, atmo_hit.x);
    var t_end = atmo_hit.y;

    // Limit by surface or scene geometry
    if max_dist > 0.0 {
        t_end = min(t_end, max_dist);
    }

    // Check if ray hits planet surface
    let planet_hit = ray_sphere(ray_origin, ray_dir, vec3(0.0), planet_radius);
    if planet_hit.x > 0.0 {
        t_end = min(t_end, planet_hit.x);
    }

    if t_end <= t_start {
        return vec3(0.0);
    }

    // Ray marching
    let segment_length = (t_end - t_start) / f32(NUM_SAMPLES);
    var total_rayleigh = vec3(0.0);
    var total_mie = vec3(0.0);
    var optical_depth_r = 0.0;
    var optical_depth_m = 0.0;

    let cos_theta = dot(ray_dir, sun_dir);
    let phase_r = rayleigh_phase(cos_theta);
    let phase_m = mie_phase(cos_theta, MIE_G);

    for (var i = 0; i < NUM_SAMPLES; i++) {
        let t = t_start + (f32(i) + 0.5) * segment_length;
        let sample_pos = ray_origin + ray_dir * t;
        let height = length(sample_pos) - planet_radius;
        let height_normalized = height / (atmo_radius - planet_radius);

        // Skip if below surface or above atmosphere
        if height_normalized < 0.0 || height_normalized > 1.0 {
            continue;
        }

        let density = atmosphere_density(height_normalized);
        let sample_optical_r = density.x * segment_length;
        let sample_optical_m = density.y * segment_length;

        optical_depth_r += sample_optical_r;
        optical_depth_m += sample_optical_m;

        // Light ray to sun
        let sun_hit = ray_sphere(sample_pos, sun_dir, vec3(0.0), atmo_radius);
        if sun_hit.y > 0.0 {
            let sun_segment = sun_hit.y / f32(NUM_LIGHT_SAMPLES);
            var light_optical_r = 0.0;
            var light_optical_m = 0.0;
            var shadow_factor = 1.0;  // 1.0 = fully lit, 0.0 = fully shadowed

            for (var j = 0; j < NUM_LIGHT_SAMPLES; j++) {
                let light_t = (f32(j) + 0.5) * sun_segment;
                let light_pos = sample_pos + sun_dir * light_t;
                let light_height = length(light_pos) - planet_radius;

                // Soft shadow: smoothly fade based on how far below surface
                // The fade distance is relative to atmosphere thickness
                let fade_distance = (atmo_radius - planet_radius) * 0.1;
                let sample_shadow = saturate(light_height / fade_distance);
                shadow_factor = min(shadow_factor, sample_shadow);

                if light_height < -fade_distance {
                    // Fully in shadow, no need to continue
                    break;
                }

                let light_height_norm = max(0.0, light_height) / (atmo_radius - planet_radius);
                let light_density = atmosphere_density(light_height_norm);
                light_optical_r += light_density.x * sun_segment;
                light_optical_m += light_density.y * sun_segment;
            }

            if shadow_factor > 0.0 {
                let tau_r = RAYLEIGH_COEFF * (optical_depth_r + light_optical_r);
                let tau_m = vec3(MIE_COEFF) * (optical_depth_m + light_optical_m);
                let attenuation = exp(-(tau_r + tau_m));

                total_rayleigh += density.x * attenuation * segment_length * shadow_factor;
                total_mie += density.y * attenuation * segment_length * shadow_factor;
            }
        }
    }

    var scatter = total_rayleigh * RAYLEIGH_COEFF * phase_r +
                  total_mie * MIE_COEFF * phase_m;

    return scatter;
}
