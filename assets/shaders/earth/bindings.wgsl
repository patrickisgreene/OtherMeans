#define_import_path earth::bindings

#import earth::params::EarthParams

// EarthMaterial::render_mode (material.rs) is the only field bound at #[uniform(0)], so Bevy's
// AsBindGroup derive exposes it as its own raw type here rather than wrapping it in a struct -
// there's no "EarthMaterial" struct on the GPU side.
@group(3) @binding(0) var<uniform> earth_material: u32;
@group(3) @binding(1) var water_normal_texture: texture_2d<f32>;
@group(3) @binding(2) var water_normal_sampler: sampler;
@group(3) @binding(3) var water_normal_2_texture: texture_2d<f32>;
@group(3) @binding(4) var water_normal_2_sampler: sampler;
@group(3) @binding(5) var<uniform> earth_constants: EarthParams;
