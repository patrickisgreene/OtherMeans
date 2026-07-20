#define_import_path earth::params

// Mirrors libraries/terrain/examples/terrain.rs::EarthConstants field-for-field (same order,
// same types) - AsBindGroup/ShaderType compute this uniform's GPU layout from the Rust struct's
// declaration order with no compile-time link to this file, so the two must be kept in lockstep
// by hand. Replaces the bare `const`s that used to live in shaders/consts.wgsl (now removed) -
// moving them here lets the debug panel adjust them live instead of requiring a shader edit.
struct EarthParams {
    ocean_transition_band: f32,

    ocean_deep_color: vec3<f32>,
    ocean_shallow_color: vec3<f32>,
    ocean_specular_color: vec3<f32>,
    ocean_fresnel_color: vec3<f32>,
    ocean_ambient: vec3<f32>,

    wave_scale_a: f32,
    wave_scale_b: f32,
    wave_strength: f32,
    wave_speed_a: f32,
    wave_speed_b: f32,
    wave_dist_ref: f32,

    spec_smoothness: f32,

    fresnel_power: f32,
    fresnel_weight: f32,

    shore_ripple_freq: f32,
    shore_ripple_speed: f32,
    shore_foam_falloff: f32,
    shore_foam_strength: f32,
    shore_foam_width: f32,
    shore_noise_scale: f32,
    shore_noise_span: f32,
}
