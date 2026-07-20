#define_import_path earth::ocean::compute

#import earth::bindings::{
    water_normal_texture, water_normal_sampler,
    water_normal_2_texture, water_normal_2_sampler,
    earth_constants
}
#import earth::ocean::triplanar_wave::triplanar_wave_sample
#import bevy_pbr::mesh_view_bindings::globals

// Two animated triplanar layers combined into a single world-space wave normal.
// Each layer samples a different normal map texture for more visual variety.
// cam_dist scales UV frequency so waves keep a consistent apparent size at any zoom.
// Called once in render() and shared with both day shading and night shimmer.
fn compute_wave_normal(sphere_normal: vec3<f32>, cam_dist: f32) -> vec3<f32> {
    let scale_mul = clamp(earth_constants.wave_dist_ref / max(cam_dist, 1.0e4), 0.25, 300.0);
    let sa    = earth_constants.wave_scale_a * scale_mul;
    let sb    = earth_constants.wave_scale_b * scale_mul;
    let t     = globals.time;
    let off_a = vec2<f32>( t * earth_constants.wave_speed_a,        t * earth_constants.wave_speed_a * 0.8);
    let off_b = vec2<f32>(-t * earth_constants.wave_speed_b * 0.8, -t * earth_constants.wave_speed_b * 0.5);
    let n_a   = triplanar_wave_sample(sphere_normal, sa, off_a, water_normal_texture,   water_normal_sampler);
    let n_b   = triplanar_wave_sample(sphere_normal, sb, off_b, water_normal_2_texture, water_normal_2_sampler);
    return normalize(n_a + n_b);
}

// Returns a 0-1 foam strength for shore ripples.
// Extracted from water_surface_color so it can be applied to both day and night.
// shore_distance is no longer a real bathymetric SDF - it's `ocean_blend * ocean_transition_band`
// (see fragment.wgsl's compute_ocean_blend), 0 at the coastline and saturating once fully out
// to sea. Kept in the same units/scale the original SDF used, so the tuning constants below
// didn't need to change.
fn compute_shore_foam(shore_distance: f32, sphere_normal: vec3<f32>, cam_dist: f32) -> f32 {
    // Scale freq and falloff so bands maintain consistent apparent size at any zoom.
    let scale_mul = clamp(earth_constants.wave_dist_ref / max(cam_dist, 1.0e4), 1.0, 200.0);
    let freq    = earth_constants.shore_ripple_freq * scale_mul;
    let falloff = earth_constants.shore_foam_falloff * scale_mul;

    // Smooth spatial noise shifts the foam phase, making lines wiggle organically
    // instead of forming perfect concentric rings.  Animates slowly so the shape
    // evolves over time.  sphere_normal.xz projects the sphere onto a 2-D plane
    // which covers equatorial coastlines well.
    let noise_coord = sphere_normal.xz * earth_constants.shore_noise_scale + globals.time * vec2<f32>(0.002, 0.001);
    let noise_phase = (smooth_noise2(noise_coord) - 0.5) * earth_constants.shore_noise_span;

    // Bands roll toward shore (phase decreases with time → troughs move to lower shore_distance).
    let phase    = shore_distance * freq - globals.time * earth_constants.shore_ripple_speed + noise_phase;
    let band_val = sin(phase); // [-1, 1]; troughs at -1 mark the foam crests

    // Narrow bright band at the trough: 1 where band_val == -1, falls to 0 over foam_width.
    let foam_line = 1.0 - smoothstep(0.0, earth_constants.shore_foam_width * 2.0, band_val + 1.0);

    // Exponential falloff from shore keeps foam near the waterline.
    let dist_fade = exp(-shore_distance * falloff);

    // Thin constant foam right at the waterline itself.
    let waterline = 1.0 - smoothstep(0.0, 1.5 / max(scale_mul, 1.0), shore_distance);

    return saturate((foam_line + waterline * 0.4) * dist_fade);
}

// Gaussian specular matching SebLague's Globe/Ocean shaders.
// view_dir points toward the camera (Bevy convention, opposite of Unity's).
fn gaussian_specular(wave_normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    let h          = normalize(light_dir + view_dir); // Blinn-Phong half-vector
    let spec_angle = acos(clamp(dot(h, wave_normal), -1.0, 1.0));
    let spec_exp   = spec_angle / earth_constants.spec_smoothness;
    return exp(-max(0.0, spec_exp) * spec_exp);
}

// 2-D value noise: smoothly interpolated random field, no texture needed.
// Returns [0, 1].  Adjacent cells blend with cubic-Hermite weights so there
// are no visible grid artefacts at the cell boundaries.
fn smooth_noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f); // cubic Hermite smoothstep
    let h = vec2<f32>(127.1, 311.7);
    let a = fract(sin(dot(i,                       h)) * 43758.5453);
    let b = fract(sin(dot(i + vec2<f32>(1.0, 0.0), h)) * 43758.5453);
    let c = fract(sin(dot(i + vec2<f32>(0.0, 1.0), h)) * 43758.5453);
    let d = fract(sin(dot(i + vec2<f32>(1.0, 1.0), h)) * 43758.5453);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn water_surface_color(
    ocean_blend:   f32,
    sphere_normal: vec3<f32>,
    view_dir:      vec3<f32>,
    light_dir:     vec3<f32>,
    wave_normal:   vec3<f32>,
) -> vec4<f32> {
    // No standalone bathymetry attachment anymore - the height texture floors the seabed to 0
    // at sea level (see resources/earth/preprocess.sh), so there's no real depth left to read.
    // Reuse the land/ocean blend itself as a depth stand-in: 0 (shallow) right at the
    // coastline, saturating to 1 (deep) once fully out in open ocean.
    var color = mix(earth_constants.ocean_shallow_color, earth_constants.ocean_deep_color, ocean_blend);

    // Wave crest shimmer: brightens faces where the wave normal is tilted away
    // from the viewer, catching ambient light at crest edges.
    // Equivalent to SebLague's dot(waveNormal, viewDir_toward_surface).
    let shimmer = saturate(smoothstep(-0.53, 0.54, -dot(wave_normal, view_dir)));
    color += shimmer * 0.12;

    // Wrap diffuse: (N·L * 0.5 + 0.5)^2 gives a soft rolloff at the terminator
    // and prevents pitch-black shading on the dark side.
    let wrap_shade = dot(sphere_normal, light_dir) * 0.5 + 0.5;
    let shading    = wrap_shade * wrap_shade;

    // Gaussian specular glint through the wave-perturbed normal.
    let spec = gaussian_specular(wave_normal, view_dir, light_dir);
    color    = saturate(color * (1.0 - spec) * shading) + spec * earth_constants.ocean_specular_color;

    // Ambient fill keeps the terminator edge dark but not black.
    color = saturate(color + earth_constants.ocean_ambient);

    // Fresnel rim: grazing angles pick up a sky-reflection tint.
    let fresnel = earth_constants.fresnel_weight * pow(1.0 - saturate(dot(sphere_normal, view_dir)), earth_constants.fresnel_power);
    color += saturate(fresnel) * earth_constants.ocean_fresnel_color;

    return vec4<f32>(color, 1.0);
}
