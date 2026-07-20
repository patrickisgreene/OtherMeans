#define_import_path earth::ocean::triplanar_wave

#import earth::bindings::earth_constants

// Decode XY tangent-space normal from rg channels; reconstruct Z.
fn unpack_normal_xy(packed: vec4<f32>, strength: f32) -> vec3<f32> {
    let xy = (packed.xy * 2.0 - 1.0) * strength;
    let z  = sqrt(max(0.0, 1.0 - dot(xy, xy)));
    return vec3<f32>(xy.x, xy.y, z);
}

// Reoriented Normal Mapping: blends tangent-space n2 onto surface n1.
// http://blog.selfshadow.com/publications/blending-in-detail/
fn blend_rnm(n1: vec3<f32>, n2: vec3<f32>) -> vec3<f32> {
    let a = vec3<f32>(n1.x, n1.y, n1.z + 1.0);
    let b = vec3<f32>(-n2.x, -n2.y, n2.z);
    return normalize(a * dot(a, b) / a.z - b);
}

// Triplanar normal sample — projects the wave normal map onto all three axes,
// reorients each tangent sample to world space via RNM, then blends by
// squared normal components (matches SebLague's Triplanar.hlsl).
fn triplanar_wave_sample(
    sphere_normal: vec3<f32>,
    scale:         f32,
    offset:        vec2<f32>,
    tex:           texture_2d<f32>,
    samp:          sampler,
) -> vec3<f32> {
    let abs_n = abs(sphere_normal);

    var w = sphere_normal * sphere_normal;
    w /= w.x + w.y + w.z;

    let uv_x = sphere_normal.zy * scale + offset;
    let uv_y = sphere_normal.xz * scale + offset;
    let uv_z = sphere_normal.xy * scale + offset;

    let tn_x = unpack_normal_xy(textureSample(tex, samp, uv_x), earth_constants.wave_strength);
    let tn_y = unpack_normal_xy(textureSample(tex, samp, uv_y), earth_constants.wave_strength);
    let tn_z = unpack_normal_xy(textureSample(tex, samp, uv_z), earth_constants.wave_strength);

    // Reorient each tangent-space sample toward its projection plane's world-space up.
    var wn_x = blend_rnm(vec3<f32>(sphere_normal.zy, abs_n.x), tn_x);
    var wn_y = blend_rnm(vec3<f32>(sphere_normal.xz, abs_n.y), tn_y);
    var wn_z = blend_rnm(vec3<f32>(sphere_normal.xy, abs_n.z), tn_z);

    let axis_sign = sign(sphere_normal);
    let fn_x = vec3<f32>(wn_x.x, wn_x.y, wn_x.z * axis_sign.x);
    let fn_y = vec3<f32>(wn_y.x, wn_y.y, wn_y.z * axis_sign.y);
    let fn_z = vec3<f32>(wn_z.x, wn_z.y, wn_z.z * axis_sign.z);

    return normalize(fn_x.zyx * w.x + fn_y.xzy * w.y + fn_z.xyz * w.z);
}
