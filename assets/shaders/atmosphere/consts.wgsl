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
