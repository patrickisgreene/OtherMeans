#import terrain::types::AtlasTile
#import terrain::bindings::{terrain, attachments, earth_attachment}
#import terrain::attachments::{sample_height, sample_surface_gradient}
#import terrain::fragment::{FragmentInput, FragmentOutput, FragmentInfo, fragment_info, fragment_debug, fragment_output}
#import terrain::functions::lookup_tile
#import bevy_pbr::mesh_view_bindings::{lights, view}
#import earth::bindings::{earth_material, earth_constants}
#import earth::render::{render_earth, sample_attachment_float}
#import earth::ocean::compute::{compute_wave_normal, compute_shore_foam, water_surface_color}

// Ocean/land split: the height texture stores an authoritative 0.0 for every ocean/lake pixel
// and a strictly-positive value for land (see resources/earth/scripts/heightmap.sh), so this
// really is just "is height zero" - a hard step, not a smoothstep. GEBCO's 8-bit source can't
// tell a flat river delta hundreds of km inland from sea level (both round to the same byte),
// so real low-lying land can end up with a height barely above 0 - a fwidth-derived band (which
// has to be floored to some small constant to avoid divide-by-zero-ish blowups on flat ground)
// is easy to accidentally make wider than that gap, which was blending huge stretches of real
// land toward "ocean" instead of just antialiasing the actual coastline. Bilinear filtering of
// the height texture already gives a soft transition right at the coastline for free; this
// doesn't need to add its own.
fn compute_ocean_blend(height: f32) -> f32 {
    return select(0.0, 1.0, height <= 0.0);
}

// Inverse of the sRGB EOTF the GPU applies when sampling earth_attachment - it's
// Rgba8UnormSrgb because its r/g/b channels carry real photographic land color, which needs
// that gamma decode. The coastline distance field packed into its blue channel over water
// (see resources/earth/scripts/distance-field.sh and earth.sh's mask-merge) isn't gamma data
// though, so the GPU decoding it anyway corrupts the byte value - this undoes that decode to
// recover what was actually written before doing any distance math on it.
fn undo_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        return value * 12.92;
    }
    return 1.055 * pow(value, 1.0 / 2.4) - 0.055;
}

// 0 on land and right at the coastline, ramping up to 1 out at sea (saturating at the SDF's own
// cap distance). Height alone can't drive this - it's flat 0.0 everywhere in open ocean - so
// this still reads the real distance field, purely to keep shore foam localized to the coast.
fn compute_shore_distance(tile: AtlasTile) -> f32 {
    let raw = sample_attachment_float(tile, earth_attachment, attachments.earth).b;
    let shore_signed = undo_srgb(raw);
    return saturate((shore_signed - 0.5) * 2.0);
}

fn render(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);
    let tile = lookup_tile(info.coordinate, info.blend);

    let light_dir = lights.directional_lights[0].direction_to_light;
    let surface_gradient = sample_surface_gradient(tile, info.tangent_space);
    let sphere_normal = normalize(info.world_coordinate.normal);
    let cam_to_frag = view.world_position - info.world_coordinate.position;
    let cam_dist = length(cam_to_frag);
    let view_dir = cam_to_frag / max(cam_dist, 1.0e-6);

    let height = sample_height(tile);
    let ocean_blend = compute_ocean_blend(height);
    let shore_distance = compute_shore_distance(tile);

    // Wave normal sampled at top level (uniform control flow) so textureSample is reachable
    // before any branching.
    let wave_normal = compute_wave_normal(sphere_normal, cam_dist);

    // Purely geometric day/night factor (no emissive/texture dependency) kept only to
    // preserve the open-ocean look below, which bypasses PBR entirely and so doesn't get a
    // day/night terminator for free the way the PBR-lit land path does.
    let ndotl = dot(sphere_normal, light_dir);
    let day_factor = smoothstep(-0.05, 0.05, ndotl);

    let foam_strength = compute_shore_foam(shore_distance, sphere_normal, cam_dist);
    let foam_color = vec3<f32>(0.88, 0.93, 1.0);

    var output: FragmentOutput;

    if ocean_blend >= 1.0 {
        // Open ocean: water_surface_color already has its own self-contained lighting model
        // (wrap diffuse + gaussian specular), so this bypasses PBR entirely - piping it through
        // fragment_output/apply_pbr_lighting below would light it a second time, creating a
        // visible brightness/color discontinuity right where ocean_blend snaps from the
        // antialiased blend zone (single-lit) to fully open ocean (previously double-lit).
        var color = water_surface_color(ocean_blend, sphere_normal, view_dir, light_dir, wave_normal);

        // Night-side ocean shimmer: wave faces tilted away from the viewer catch a
        // subtle ambient brightening. Purely view-angle based — no sun required.
        let shimmer = saturate(smoothstep(-0.53, 0.54, -dot(wave_normal, view_dir))) * 0.05;
        color = mix(color, vec4<f32>(color.rgb + shimmer, 1.0), 1.0 - day_factor);

        // Full brightness on day side, 20% on night side (just visible in moonlight).
        let foam_day_factor = mix(0.20, 1.0, day_factor);
        color = vec4<f32>(mix(color.rgb, foam_color, foam_strength * earth_constants.shore_foam_strength * foam_day_factor), 1.0);

        output.color = color;
        fragment_debug(&info, &output, tile, surface_gradient);
        return output;
    }

    // Land (and the coastline blend toward ocean): day-lit albedo fed through real PBR
    // lighting - the day/night terminator emerges from N·L.
    let land_color = render_earth(tile, info, surface_gradient, wave_normal, ocean_blend, sphere_normal, view_dir, light_dir);

    fragment_output(&info, &output, land_color, surface_gradient);

    // Shore foam applied on top of the PBR-lit result; brightness across the terminator now
    // comes from PBR's N·L rather than a manual day/night factor.
    output.color = vec4<f32>(mix(output.color.rgb, foam_color, foam_strength * earth_constants.shore_foam_strength * ocean_blend), 1.0);

    fragment_debug(&info, &output, tile, surface_gradient);
    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;
    var info = fragment_info(input);
    let tile = lookup_tile(info.coordinate, info.blend);
    let height = sample_height(tile);
    switch earth_material {
        case 1: { // Height
            output.color = vec4(vec3(saturate(height / terrain.max_height)), 1.0);
        }
        case 2: { // Ocean Mask
            output.color = vec4(vec3(compute_ocean_blend(height)), 1.0);
        }
        case 3: { // Color
            let sample_color = sample_attachment_float(tile, earth_attachment, attachments.earth).rgb;
            let color = mix(sample_color, vec3(0.0), compute_ocean_blend(height));
            output.color = vec4(color, 1.0);
        }
        case 4: { // Distance - the real coastline SDF read from earth_attachment's blue channel.
            output.color = vec4(vec3(compute_shore_distance(tile)), 1.0);
            let color = mix(0.0, compute_shore_distance(tile), compute_ocean_blend(height));
            output.color = vec4(0.0, 0.0, color, 1.0);
        }
        case 5: { // Bathyometry
            let sample_color = sample_attachment_float(tile, earth_attachment, attachments.earth).r;
            let color = mix(0.0, sample_color, compute_ocean_blend(height));
            output.color = vec4(vec3(color), 1.0);
        }
        case 6: { // Chlorophyll
            let sample_color = sample_attachment_float(tile, earth_attachment, attachments.earth).g;
            let color = mix(0.0, sample_color,compute_ocean_blend(height));
            output.color = vec4(0.0, color, 0.0, 1.0);
        }
        default: {
            return render(input);
        }
    }
    return output;
}
