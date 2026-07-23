#import "shaders/atmosphere/consts.wgsl"::PI
#import "shaders/atmosphere/bindings.wgsl"::{settings, wind_texture, wind_sampler}

// ============================================================================
// Clouds
// ============================================================================
//
// A 2D cloud shell at settings.cloud_altitude_scale (a fixed multiple of the planet radius,
// same convention as atmosphere_radius_scale), shaded with 3D Perlin-noise FBM sampled directly
// on the sphere direction (not an equirectangular UV) so there's no seam at the antimeridian and
// no pinching at the poles - only the wind_texture lookup that advects it uses lat/lon UVs,
// since a visible seam in the (much lower-frequency, purely directional) driving wind field is
// imperceptible once it's absorbed into the seamless 3D noise it's offsetting.

// Perlin-style 3D gradient noise. Gradients aren't unit-length (cheap hash instead of a proper
// gradient table) - this drifts the output range a bit off the textbook [-1, 1], which doesn't
// matter here since cloud density only ever cares about relative shape, not an exact range.
fn hash3(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx) * 2.0 - 1.0;
}

fn perlin_noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0); // quintic fade

    let n000 = dot(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), f - vec3<f32>(0.0, 0.0, 0.0));
    let n100 = dot(hash3(i + vec3<f32>(1.0, 0.0, 0.0)), f - vec3<f32>(1.0, 0.0, 0.0));
    let n010 = dot(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), f - vec3<f32>(0.0, 1.0, 0.0));
    let n110 = dot(hash3(i + vec3<f32>(1.0, 1.0, 0.0)), f - vec3<f32>(1.0, 1.0, 0.0));
    let n001 = dot(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), f - vec3<f32>(0.0, 0.0, 1.0));
    let n101 = dot(hash3(i + vec3<f32>(1.0, 0.0, 1.0)), f - vec3<f32>(1.0, 0.0, 1.0));
    let n011 = dot(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), f - vec3<f32>(0.0, 1.0, 1.0));
    let n111 = dot(hash3(i + vec3<f32>(1.0, 1.0, 1.0)), f - vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, u.x);
    let nx10 = mix(n010, n110, u.x);
    let nx01 = mix(n001, n101, u.x);
    let nx11 = mix(n011, n111, u.x);

    let nxy0 = mix(nx00, nx10, u.y);
    let nxy1 = mix(nx01, nx11, u.y);

    return mix(nxy0, nxy1, u.z);
}

// 5-octave fractal sum - each octave doubles frequency and halves amplitude, building the
// layered, wispy detail actual cloud shapes have instead of one smooth blob scale.
fn cloud_fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var freq_p = p;
    for (var i = 0; i < 5; i++) {
        value += amplitude * perlin_noise3(freq_p);
        freq_p *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// dir is a normalized direction from the planet center (i.e. a point on the unit sphere).
fn direction_to_latlon(dir: vec3<f32>) -> vec2<f32> {
    let lat = asin(clamp(dir.y, -1.0, 1.0));
    let lon = atan2(dir.z, dir.x);
    return vec2<f32>(lon, lat);
}

// Wind direction at a point on the cloud shell, decoded from wind-flow.png's R/G channels
// (see assets/textures/earth/wind-flow.png and its generator) back from [0, 1] to [-1, 1].
fn sample_wind(dir: vec3<f32>) -> vec2<f32> {
    let latlon = direction_to_latlon(dir);
    let uv = vec2<f32>(latlon.x / (2.0 * PI) + 0.5, 0.5 - latlon.y / PI);
    let raw = textureSampleLevel(wind_texture, wind_sampler, uv, 0.0).rg;
    return raw * 2.0 - 1.0;
}

// Cloud density (0 = clear, 1 = fully opaque) at a point on the cloud shell.
fn compute_cloud_density(dir: vec3<f32>) -> f32 {
    let wind = sample_wind(dir);

    // Local east/north tangent basis at dir, used to turn the 2D wind vector into a 3D
    // advection offset. cross(up, dir) degenerates to zero at the poles, where "east" has no
    // well-defined direction - rather than patching the direction (which, fed into an
    // unbounded time integral below, previously produced a visible vortex/spiral artifact),
    // fade the wind offset itself to zero as dir approaches either pole. pole_closeness is
    // length(cross(up, dir)), i.e. cos(latitude): 1 at the equator, 0 at the poles.
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let east_raw = cross(up, dir);
    let pole_closeness = length(east_raw);
    let east = east_raw / max(pole_closeness, 1.0e-4);
    let north = cross(dir, east);
    let pole_fade = smoothstep(0.0, 0.05, pole_closeness);
    let offset = (east * wind.x + north * wind.y) * settings.time * settings.cloud_speed * pole_fade;

    let noise = cloud_fbm(dir * settings.cloud_scale + offset);
    let noise01 = saturate(noise * 0.5 + 0.5);

    let edge = 0.15 / max(settings.cloud_density, 0.05);
    return smoothstep(settings.cloud_coverage, settings.cloud_coverage + edge, noise01);
}
