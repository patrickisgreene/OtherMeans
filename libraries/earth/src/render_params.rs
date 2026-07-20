use bevy::{prelude::*, render::render_resource::ShaderType};

/// All the tuning constants that used to live as bare `const`s in `assets/shaders/consts.wgsl`,
/// now uniform fields so the debug panel can adjust them live instead of requiring a shader
/// edit + recompile. Field order/types here must exactly match the `EarthConstants` WGSL
/// struct in `assets/shaders/bindings.wgsl` - `AsBindGroup`/`ShaderType` compute GPU layout
/// (std140-style alignment: each `Vec3` occupies 16 bytes) from Rust field declaration order,
/// and there's no compile-time check tying that to the hand-written WGSL struct, so keep the
/// two in lockstep by hand when adding/removing/reordering fields.
#[derive(ShaderType, Clone, Debug)]
pub struct EarthParams {
    // Ocean/land split (shaders/earth/fragment.wgsl) - half-width, in metres, of the smoothstep
    // band around sea level (height = 0) used to derive ocean_blend from the merged height
    // attachment, replacing the old binary `surface` mask.
    pub ocean_transition_band: f32,

    // Ocean colors (shaders/earth/water.wgsl)
    pub ocean_deep_color: Vec3,
    pub ocean_shallow_color: Vec3,
    pub ocean_specular_color: Vec3,
    pub ocean_fresnel_color: Vec3,
    pub ocean_ambient: Vec3,

    // Wave normal (shaders/earth/water.wgsl)
    pub wave_scale_a: f32,
    pub wave_scale_b: f32,
    pub wave_strength: f32,
    pub wave_speed_a: f32,
    pub wave_speed_b: f32,
    pub wave_dist_ref: f32,

    // Specular (shaders/earth/water.wgsl)
    pub spec_smoothness: f32,

    // Fresnel (shaders/earth/water.wgsl)
    pub fresnel_power: f32,
    pub fresnel_weight: f32,

    // Shore foam (shaders/earth/water.wgsl)
    pub shore_ripple_freq: f32,
    pub shore_ripple_speed: f32,
    pub shore_foam_falloff: f32,
    pub shore_foam_strength: f32,
    pub shore_foam_width: f32,
    pub shore_noise_scale: f32,
    pub shore_noise_span: f32,
}

impl Default for EarthParams {
    fn default() -> Self {
        Self {
            ocean_transition_band: 5.0,

            ocean_deep_color: Vec3::new(0.007, 0.018, 0.090),
            ocean_shallow_color: Vec3::new(0.022, 0.072, 0.180) * 1.2,
            ocean_specular_color: Vec3::new(1.0, 1.0, 1.0),
            ocean_fresnel_color: Vec3::new(0.08, 0.18, 0.35),
            ocean_ambient: Vec3::new(0.003, 0.008, 0.025),

            wave_scale_a: 8.0,
            wave_scale_b: 13.0,
            wave_strength: 0.50,
            wave_speed_a: 0.010,
            wave_speed_b: 0.07,
            wave_dist_ref: 5.0e6,

            spec_smoothness: 0.030,

            fresnel_power: 4.5,
            fresnel_weight: 0.45,

            shore_ripple_freq: 42.0,
            shore_ripple_speed: 2.5,
            shore_foam_falloff: 8.5,
            shore_foam_strength: 0.55,
            shore_foam_width: 0.12,
            shore_noise_scale: 400.0,
            shore_noise_span: 12.5,
        }
    }
}
