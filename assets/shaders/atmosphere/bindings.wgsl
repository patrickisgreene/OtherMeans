#import bevy_render::view::View

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var depth_texture: texture_depth_multisampled_2d;
@group(0) @binding(3) var<uniform> view: View;

struct AtmosphereSettings {
    // Split f64 values for precision: high + low = full value
    // Fields are ordered for 16-byte alignment (vec3 + f32 = 16 bytes)
    planet_center_high: vec3<f32>,
    planet_scale: f32,
    planet_center_low: vec3<f32>,
    atmosphere_radius_scale: f32,
    sun_position: vec3<f32>,
    ambient_scatter_strength: f32,
    camera_position_high: vec3<f32>,
    //_padding3: f32,
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
@group(0) @binding(4) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(5) var wind_texture: texture_2d<f32>;
@group(0) @binding(6) var wind_sampler: sampler;
