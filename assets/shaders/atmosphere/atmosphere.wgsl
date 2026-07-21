// Shader for post-processing effects with 64-bit precision support
// Implements atmospheric scattering for Earth-scale rendering

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import "shaders/atmosphere/consts.wgsl"::MAX_FLOAT
#import "shaders/atmosphere/functions/clouds.wgsl"::compute_cloud_density
#import "shaders/atmosphere/functions/ray_sphere.wgsl"::ray_sphere
#import "shaders/atmosphere/functions/ambient_scatter.wgsl"::calculate_ambient_scatter
#import "shaders/atmosphere/functions/atmosphere.wgsl"::{hash2to1, calculate_atmosphere}
#import "shaders/atmosphere/functions/transmittance.wgsl"::calculate_transmittance
#import "shaders/atmosphere/functions/helpers.wgsl"::{get_ray_direction, frag_coord_to_ndc, sub_split_vec3}
#import "shaders/atmosphere/bindings.wgsl"::{depth_texture, settings, screen_texture, texture_sampler}

const pi_f32 = 3.141592653589;

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
