// Shader for post-processing effects with 64-bit precision support
// Implements atmospheric scattering for Earth-scale rendering

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var depth_texture: texture_depth_multisampled_2d;
@group(0) @binding(3) var<uniform> view: View;

struct PostProcessSettings {
    // Split f64 values for precision: high + low = full value
    // Fields are ordered for 16-byte alignment (vec3 + f32 = 16 bytes)
    planet_center_high: vec3<f32>,
    planet_scale: f32,
    planet_center_low: vec3<f32>,
    atmosphere_radius_scale: f32,
    sun_position: vec3<f32>,
    ambient_scatter_strength: f32,
    camera_position_high: vec3<f32>,
    _padding3: f32,
    camera_position_low: vec3<f32>,
    time: f32,
    proj_mat: mat4x4<f32>,
    inverse_proj: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    inverse_view: mat4x4<f32>,

    cloud_color: vec3<f32>,
    cloud_coverage: f32,
    cloud_altitude_scale: f32,
    cloud_scale: f32,
    cloud_speed: f32,
    cloud_density: f32,
}
@group(0) @binding(4) var<uniform> settings: PostProcessSettings;
@group(0) @binding(5) var wind_texture: texture_2d<f32>;
@group(0) @binding(6) var wind_sampler: sampler;

// ============================================================================
// Constants
// ============================================================================

const PI: f32 = 3.14159265359;
const MAX_FLOAT: f32 = 3.402823466e+38;

// Atmosphere parameters
const RAYLEIGH_SCALE_HEIGHT: f32 = 0.05;  // Normalized to atmosphere thickness
const MIE_SCALE_HEIGHT: f32 = 0.012;

// Scattering coefficients (these work in normalized space)
const RAYLEIGH_COEFF: vec3<f32> = vec3<f32>(5.8e-3, 1.35e-2, 3.31e-2);
const MIE_COEFF: f32 = 2.0e-2;
const MIE_G: f32 = 0.76;  // Mie scattering direction (-0.99 to 0.99)

// Ambient/night side scattering
const AMBIENT_SCATTER_COLOR: vec3<f32> = vec3<f32>(0.1, 0.15, 0.3);  // Slight blue tint

const NUM_SAMPLES: i32 = 8;
const NUM_LIGHT_SAMPLES: i32 = 4;

const pi_f32 = 3.141592653589;

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

// ============================================================================
// Clouds
// ============================================================================
//
// A 2D cloud shell at settings.cloud_altitude_scale (a fixed multiple of the planet radius,
// same convention as atmosphere_radius_scale), shaded with 3D Perlin-noise FBM sampled directly
// on the sphere direction (not an equirectangular UV) so there's no seam at the antimeridian and
// no pinching at the poles - only the wind_texture lookup that advects it uses lat/lon UVs,
// since a visible seam in the (much lower-frequency, purely directional) driving wind field is
// imperceptible once it's absorbed into the seamless 3D noise it's offsetting.

// Perlin-style 3D gradient noise. Gradients aren't unit-length (cheap hash instead of a proper
// gradient table) - this drifts the output range a bit off the textbook [-1, 1], which doesn't
// matter here since cloud density only ever cares about relative shape, not an exact range.
fn hash3(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx) * 2.0 - 1.0;
}

fn perlin_noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0); // quintic fade

    let n000 = dot(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), f - vec3<f32>(0.0, 0.0, 0.0));
    let n100 = dot(hash3(i + vec3<f32>(1.0, 0.0, 0.0)), f - vec3<f32>(1.0, 0.0, 0.0));
    let n010 = dot(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), f - vec3<f32>(0.0, 1.0, 0.0));
    let n110 = dot(hash3(i + vec3<f32>(1.0, 1.0, 0.0)), f - vec3<f32>(1.0, 1.0, 0.0));
    let n001 = dot(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), f - vec3<f32>(0.0, 0.0, 1.0));
    let n101 = dot(hash3(i + vec3<f32>(1.0, 0.0, 1.0)), f - vec3<f32>(1.0, 0.0, 1.0));
    let n011 = dot(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), f - vec3<f32>(0.0, 1.0, 1.0));
    let n111 = dot(hash3(i + vec3<f32>(1.0, 1.0, 1.0)), f - vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, u.x);
    let nx10 = mix(n010, n110, u.x);
    let nx01 = mix(n001, n101, u.x);
    let nx11 = mix(n011, n111, u.x);

    let nxy0 = mix(nx00, nx10, u.y);
    let nxy1 = mix(nx01, nx11, u.y);

    return mix(nxy0, nxy1, u.z);
}

// 5-octave fractal sum - each octave doubles frequency and halves amplitude, building the
// layered, wispy detail actual cloud shapes have instead of one smooth blob scale.
fn cloud_fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var freq_p = p;
    for (var i = 0; i < 5; i++) {
        value += amplitude * perlin_noise3(freq_p);
        freq_p *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// dir is a normalized direction from the planet center (i.e. a point on the unit sphere).
fn direction_to_latlon(dir: vec3<f32>) -> vec2<f32> {
    let lat = asin(clamp(dir.y, -1.0, 1.0));
    let lon = atan2(dir.z, dir.x);
    return vec2<f32>(lon, lat);
}

// Wind direction at a point on the cloud shell, decoded from wind-flow.png's R/G channels
// (see assets/textures/earth/wind-flow.png and its generator) back from [0, 1] to [-1, 1].
fn sample_wind(dir: vec3<f32>) -> vec2<f32> {
    let latlon = direction_to_latlon(dir);
    let uv = vec2<f32>(latlon.x / (2.0 * PI) + 0.5, 0.5 - latlon.y / PI);
    let raw = textureSampleLevel(wind_texture, wind_sampler, uv, 0.0).rg;
    return raw * 2.0 - 1.0;
}

// Cloud density (0 = clear, 1 = fully opaque) at a point on the cloud shell.
fn compute_cloud_density(dir: vec3<f32>) -> f32 {
    let wind = sample_wind(dir);

    // Local east/north tangent basis at dir, used to turn the 2D wind vector into a 3D
    // advection offset - degenerates at the poles (cross(up, dir) -> 0), nudged off zero since
    // clouds directly over the poles are a visually negligible edge case, not worth branching on.
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let east = normalize(cross(up, dir) + vec3<f32>(1.0e-4, 0.0, 0.0));
    let north = cross(dir, east);
    let offset = (east * wind.x + north * wind.y) * settings.time * settings.cloud_speed;

    let noise = cloud_fbm(dir * settings.cloud_scale + offset);
    let noise01 = saturate(noise * 0.5 + 0.5);

    let edge = 0.15 / max(settings.cloud_density, 0.05);
    return smoothstep(settings.cloud_coverage, settings.cloud_coverage + edge, noise01);
}

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

// ============================================================================
// Fragment shader
// ============================================================================

@fragment
fn fragment(
    @builtin(sample_index) sample_index: u32,
    in: FullscreenVertexOutput
) -> @location(0) vec4<f32> {
    // Sample the original rendered scene
    let original_color = textureSample(screen_texture, texture_sampler, in.uv);

    // Compute ray direction from screen position
    let ndc = frag_coord_to_ndc(in.position);
    let ray_dir = get_ray_direction(ndc.xy);
    let sun_dir = normalize(settings.sun_position);

    // Get planet parameters
    let planet_radius = settings.planet_scale / 2.0;

    // Camera position relative to planet center (in world units)
    let camera_world = sub_split_vec3(
        settings.camera_position_high, settings.camera_position_low,
        settings.planet_center_high, settings.planet_center_low
    );

    // Normalize everything to planet radius = 1.0 for numerical stability
    let scale = 1.0 / planet_radius;
    let camera_normalized = camera_world * scale;
    let planet_r_norm = 1.0;
    let atmo_r_norm = settings.atmosphere_radius_scale;

    // Read raw depth from depth buffer (reverse-z: 0 = far, 1 = near)
    let raw_depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), i32(sample_index));

    // Check if there's geometry in the depth buffer
    let has_geometry = raw_depth > 0.0 && raw_depth < 1.0;

    // Reconstruct terrain ray distance from depth buffer
    var atmo_dist = -1.0;
    if has_geometry {
        // Reconstruct view-space position from depth
        let clip_pos = vec4<f32>(ndc.x, ndc.y, raw_depth, 1.0);
        let view_pos_h = settings.inverse_proj * clip_pos;
        let view_pos = view_pos_h.xyz / view_pos_h.w;

        // Get ray distance (length from camera to point), not just z-depth
        // Then convert to normalized space
        atmo_dist = length(view_pos) * scale;
    }

    // Calculate atmosphere scattering
    let scatter = calculate_atmosphere(
        camera_normalized,
        ray_dir,
        sun_dir,
        atmo_dist,
        raw_depth,
        planet_r_norm,
        atmo_r_norm
    );

    // Calculate transmittance for original color
    let transmittance = calculate_transmittance(
        camera_normalized,
        ray_dir,
        atmo_dist,
        planet_r_norm,
        atmo_r_norm
    );

    // Calculate ambient scattering for night side
    let ambient_scatter = calculate_ambient_scatter(
        camera_normalized,
        ray_dir,
        atmo_dist,
        planet_r_norm,
        atmo_r_norm
    );

    // Combine: sun scatter + ambient scatter (ambient is always added, sun scatter dominates on day side)
    let total_scatter = scatter * 1000.0 + ambient_scatter;

    // Sun and stars (rendered for all sky pixels, independent of atmosphere)
    var stars = vec3(0.0);
    if raw_depth < 0.000001 {
        let planet_hit = ray_sphere(camera_normalized, ray_dir, vec3(0.0), planet_r_norm);
        if planet_hit.x < 0.0 {
            // Sun disc
            let sun_cos = dot(ray_dir, -sun_dir);
            let sun_disc = smoothstep(0.9999, 0.99999, sun_cos);
            let sun_color = vec3(1.0, 0.85, 0.6);
            stars += sun_disc * sun_color;

            // Stars
            var psi = 0.;
            var phi = 0.;
            if abs(ray_dir.y) > abs(ray_dir.z) && abs(ray_dir.y) > abs(ray_dir.x) {
                psi = atan(ray_dir.y / ray_dir.x);
                phi = atan(ray_dir.y / ray_dir.z);
            } else {
                if abs(ray_dir.x) > abs(ray_dir.z) {
                    psi = atan(ray_dir.x / ray_dir.y);
                    phi = atan(ray_dir.x / ray_dir.z);
                } else {
                    psi = atan(ray_dir.z / ray_dir.x);
                    phi = atan(ray_dir.z / ray_dir.y);
                }
            }
            let px = psi * 60. / pi_f32;
            let py = phi * 60. / pi_f32;
            let tile_n = vec2<f32>(floor(px), floor(py));
            let tile_pos = vec2<f32>(fract(px), fract(py));
            let hash_n = hash2to1(tile_n);
            let star_bright = max(1. - 60. * (sin(hash_n) * 0.5 + 0.5) * distance(tile_pos, vec2(hash_n + 0.5, fract(hash_n * 10.))), 0.);
            stars += vec3(
                star_bright * (0.8 + sin(fract(hash_n * 10.)) * 0.2),
                star_bright * (0.6 + sin(fract(hash_n * 100.)) * 0.4),
                star_bright * (0.4 + sin(fract(hash_n * 1000.)) * 0.6));
        }
    }

    // Clouds: a thin shell just above the surface, shaded with real Perlin-noise FBM advected
    // by the small static wind-flow texture (see the "Clouds" section above). Only rendered
    // where the ray actually reaches the shell before anything closer - real geometry, or the
    // planet's own analytic sphere - would occlude it.
    var cloud_color = vec3(0.0);
    var cloud_alpha = 0.0;
    let cloud_r_norm = settings.cloud_altitude_scale;
    let cloud_hit = ray_sphere(camera_normalized, ray_dir, vec3(0.0), cloud_r_norm);
    if cloud_hit.y > 0.0 {
        let t_cloud = max(cloud_hit.x, 0.0);
        let ground_dist = select(MAX_FLOAT, atmo_dist, has_geometry);
        if t_cloud < ground_dist {
            let cloud_pos = camera_normalized + ray_dir * t_cloud;
            let cloud_dir = normalize(cloud_pos);
            let density = compute_cloud_density(cloud_dir);

            // Soft terminator, matching the day/night bands used elsewhere (ocean, sun disc) -
            // a small ambient floor keeps night-side clouds visible rather than pure black.
            let sun_dot = dot(cloud_dir, sun_dir);
            let day_factor = smoothstep(-0.2, 0.1, -sun_dot);
            cloud_color = settings.cloud_color * mix(0.04, 1.0, day_factor);
            cloud_alpha = density * 0.9;
        }
    }

    // Combine with original scene color
    let final_color = mix(
        original_color.rgb * transmittance + total_scatter + stars,
        cloud_color,
        cloud_alpha
    );

    return vec4(final_color, 1.0);
}
