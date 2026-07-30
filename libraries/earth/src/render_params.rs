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
    /// Tint blended in over chlorophyll-rich (coastal upwelling / river plume) water.
    pub ocean_chlorophyll_color: Vec3,
    /// Base ocean color blended in toward the poles (high |latitude|) - a deeper, colder blue.
    pub ocean_polar_color: Vec3,
    /// Base ocean color blended in toward the equator (low |latitude|) - a lighter, warmer blue.
    pub ocean_equatorial_color: Vec3,
    /// Brightness contrast applied from the real GEBCO bathymetric hillshade (earth_attachment's ocean-side
    /// red channel), centered on its ~0.7 mean so typical seafloor renders at neutral
    /// brightness. 0 = no relief shading (flat color), higher = more ridge/trench contrast.
    pub bathymetry_strength: f32,
    /// Multiplier on the raw chlorophyll byte (earth_attachment's green channel on the ocean side, mean ~0.03)
    /// before it saturates into the chlorophyll tint - real concentrations are low outside
    /// productive coastal zones, so this needs to be well above 1 to be visible at all.
    pub chlorophyll_strength: f32,
    /// How strongly the polar/equatorial latitude tint blends into the base ocean color -
    /// 0 disables it, 1 fully replaces the depth-based shallow/deep color with the latitude tint.
    pub ocean_latitude_strength: f32,

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

            ocean_deep_color: Vec3::new(0.010, 0.035, 0.110),
            ocean_shallow_color: Vec3::new(0.035, 0.130, 0.260),
            ocean_specular_color: Vec3::new(1.0, 1.0, 1.0),
            ocean_fresnel_color: Vec3::new(0.08, 0.18, 0.35),
            ocean_ambient: Vec3::new(0.003, 0.008, 0.025),
            ocean_chlorophyll_color: Vec3::new(0.055, 0.150, 0.095),
            ocean_polar_color: Vec3::new(0.010, 0.045, 0.140),
            ocean_equatorial_color: Vec3::new(0.060, 0.220, 0.360),
            bathymetry_strength: 1.2,
            chlorophyll_strength: 3.0,
            ocean_latitude_strength: 0.4,

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
            shore_foam_strength: 2.0,
            shore_foam_width: 0.12,
            shore_noise_scale: 400.0,
            shore_noise_span: 12.5,
        }
    }
}
