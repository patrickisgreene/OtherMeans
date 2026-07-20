#define_import_path earth::render

#import terrain::types::{AtlasTile, AttachmentConfig}
#import terrain::bindings::{attachments, terrain_sampler, earth_attachment}
#import terrain::attachments::{compute_sample_uv, relief_shading}
#import terrain::fragment::FragmentInfo
#import earth::ocean::compute::water_surface_color

fn sample_attachment_float(tile: AtlasTile, attachment: texture_2d_array<f32>, config: AttachmentConfig) -> vec4<f32> {
    let uv = compute_sample_uv(tile, config);
    return textureSampleLevel(attachment, terrain_sampler, uv.uv, tile.index, tile.blend_ratio);
}

// ocean_blend is computed once in shaders/earth/fragment.wgsl (compute_ocean_blend) and passed
// in here. render() (fragment.wgsl) handles the pure-open-ocean case (ocean_blend >= 1.0)
// itself, before ever calling this function, so that water_surface_color's self-contained
// lighting model can bypass PBR entirely instead of being lit twice - so this only ever runs
// for land or the land/ocean blend zone.
fn render_earth(tile: AtlasTile, info: FragmentInfo, surface_gradient: vec3<f32>, wave_normal: vec3<f32>,
       ocean_blend: f32, sphere_normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>) -> vec4<f32> {
    let sample_color = sample_attachment_float(tile, earth_attachment, attachments.earth).rgb;
    var land_color = vec4<f32>(sample_color, 1.0);
    // Fade relief shading toward neutral near water to avoid dark halos at island edges.
    let relief = mix(1.0, relief_shading(info.world_coordinate, surface_gradient), 1.0 - ocean_blend);
    land_color = land_color * relief;

    if ocean_blend <= 0.0 {
        return land_color;
    }

    let ocean_color = water_surface_color(ocean_blend, sphere_normal, view_dir, light_dir, wave_normal);
    return mix(land_color, ocean_color, ocean_blend);
}
