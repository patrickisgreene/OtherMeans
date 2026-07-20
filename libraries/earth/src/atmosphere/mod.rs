mod pass;
mod pipeline;
mod plugin;
mod systems;

pub use plugin::EarthAtmospherePlugin;

use bevy::{
    math::DVec3,
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};

/// Splits a f64 value into two f32 values for GPU precision.
/// The high part contains the rounded value, the low part contains the remainder.
/// This allows reconstruction of ~48 bits of precision on the GPU.
pub fn split_f64(value: f64) -> (f32, f32) {
    let high = value as f32;
    let low = (value - high as f64) as f32;
    (high, low)
}

/// Splits a DVec3 into two Vec3s for GPU precision.
pub fn split_dvec3(value: DVec3) -> (Vec3, Vec3) {
    let (x_high, x_low) = split_f64(value.x);
    let (y_high, y_low) = split_f64(value.y);
    let (z_high, z_low) = split_f64(value.z);
    (
        Vec3::new(x_high, y_high, z_high),
        Vec3::new(x_low, y_low, z_low),
    )
}

// This is the component that will get passed to the shader
// IMPORTANT: Field order must match WGSL struct exactly, respecting 16-byte alignment
// vec3 + f32 = 16 bytes (f32 fills the vec3 padding)
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct EarthAtmosphereSettings {
    /// High bits of planet center (f64 split)
    pub planet_center_high: Vec3,
    /// Planet scale (diameter) - packed with planet_center_high for alignment
    pub planet_scale: f32,
    /// Low bits of planet center (f64 split)
    pub planet_center_low: Vec3,
    /// Padding to align next vec3
    pub atmosphere_radius_scale: f32,
    /// Sun position in world space
    pub sun_position: Vec3,
    /// Padding to align next vec3
    pub ambient_scatter_strength: f32,
    /// High bits of camera position (f64 split)
    pub camera_position_high: Vec3,
    /// Padding to align next vec3
    pub _padding3: f32,
    /// Low bits of camera position (f64 split)
    pub camera_position_low: Vec3,
    /// Padding to align mat4
    pub _padding4: f32,
    pub proj_mat: Mat4,
    pub inverse_proj: Mat4,
    pub view_mat: Mat4,
    pub inverse_view: Mat4,
}

impl EarthAtmosphereSettings {
    /// Set the camera position using 64-bit precision
    pub fn set_camera_position(&mut self, position: DVec3) {
        let (high, low) = split_dvec3(position);
        self.camera_position_high = high;
        self.camera_position_low = low;
    }

    /// Set the planet center using 64-bit precision
    pub fn set_planet_center(&mut self, center: DVec3) {
        let (high, low) = split_dvec3(center);
        self.planet_center_high = high;
        self.planet_center_low = low;
    }
}

pub const SHADER_ASSET_PATH: &str = "shaders/atmosphere.wgsl";

impl Default for EarthAtmosphereSettings {
    fn default() -> EarthAtmosphereSettings {
        EarthAtmosphereSettings {
            planet_center_high: Default::default(),
            planet_scale: Default::default(),
            planet_center_low: Default::default(),
            atmosphere_radius_scale: 1.125,
            sun_position: Default::default(),
            ambient_scatter_strength: 5.0,
            camera_position_high: Default::default(),
            _padding3: Default::default(),
            camera_position_low: Default::default(),
            _padding4: Default::default(),
            proj_mat: Default::default(),
            inverse_proj: Default::default(),
            view_mat: Default::default(),
            inverse_view: Default::default(),
        }
    }
}
