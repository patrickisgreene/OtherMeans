use bevy::feathers::controls::{ButtonVariant, FeathersToolButton};
use bevy::feathers::theme::ThemedText;
use bevy::feathers::{constants::icons, containers::*, display::*};
use terrain::debug::DebugTerrain;

use crate::debug::systems::disclosure_toggle;

use super::*;

fn remove_debug_panel(
    _: On<Pointer<Click>>,
    mut commands: Commands,
    query: Query<Entity, With<DebugPanel>>,
) {
    commands.entity(query.single().unwrap()).despawn();
}

pub fn debug_panel() -> impl Scene {
    bsn! {
        #DebugPanel
        DebugPanel
        Node {
            right: px(0),
            display: Display::Flex,
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
        }
        Children [
            (
                pane() Children [
                    pane_header() Children [
                        @FeathersToolButton {
                            @variant: ButtonVariant::Primary,
                            @caption: bsn! { Text("\u{0398}") ThemedText }
                        },
                        pane_header_divider(),
                        label("Earth"),
                        flex_spacer(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                            @caption: bsn! { on(remove_debug_panel) icon(icons::X) }
                        },
                    ],
                    (
                        pane_body() Children [
                            subpane() Children [
                                subpane_header() Children [
                                    (Text("Terrain") ThemedText),
                                ],
                                subpane_body() Children [
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<LodGroupBody>(),
                                            (Text("Level Of Detail") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            LodGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                // TerrainViewConfig::default() values (view.rs) - these are
                                                // face-size-relative multipliers, but TileTree::new() stores
                                                // them already multiplied by face_size (~1e7m for Earth) on
                                                // the live TileTree (morph/blend/load_distance). Setters must
                                                // re-multiply by face_size, or they overwrite a ~10-million-metre
                                                // field with a tiny 0.25-20 value, which starves every LOD except
                                                // the always-requested coarsest one (lod 0) of tile requests.
                                                lod_slider("Morph Distance", 1.0, 50.0, 0.5, 40.0, |tile_tree, value| {tile_tree.morph_distance = value * tile_tree.shape.face_size();}),
                                                lod_slider("Blend Distance", 0.25, 20.0, 0.25, 5.0, |tile_tree, value| {tile_tree.blend_distance = value * tile_tree.shape.face_size();}),
                                                lod_slider("Load Distance", 0.25, 20.0, 0.25, 6.0, |tile_tree, value| {tile_tree.load_distance = value * tile_tree.shape.face_size();}),
                                                lod_slider("Grid Size", 2.0, 64.0, 2.0, 16.0, |tile_tree, value| {tile_tree.grid_size = value as u32;}),
                                            ]
                                        ),
                                    ],
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<TerrainGroupBody>(),
                                            (Text("Terrain") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            TerrainGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                // Defaults match assets/textures/earth/earth.ron
                                                // (real-world metres recorded during preprocessing).
                                                terrain_slider("Min Height", -20000.0, 0.0, 100.0, -10752.082, |tile_atlas, value| {tile_atlas.min_height = value;}),
                                                terrain_slider("Max Height", 0.0, 20000.0, 100.0, 8157.356, |tile_atlas, value| {tile_atlas.max_height = value;}),
                                                terrain_slider("Height Scale", 0.0, 40000.0, 500.0, 18909.438, |tile_atlas, value| {tile_atlas.height_scale = value;}),
                                            ]
                                        ),
                                    ],
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<RenderingGroupBody>(),
                                            (Text("Rendering") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            RenderingGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                // DebugTerrain::default() values (debug/mod.rs) -
                                                // morph/blend default on, everything else off.
                                                debug_switch("Wireframe", |d: &mut DebugTerrain, v| d.wireframe = v),
                                                debug_switch("Show Data LOD", |d: &mut DebugTerrain, v| d.show_data_lod = v),
                                                debug_switch("Show Geometry LOD", |d: &mut DebugTerrain, v| d.show_geometry_lod = v),
                                                debug_switch("Show Tile Tree", |d: &mut DebugTerrain, v| d.show_tile_tree = v),
                                                debug_switch("Show Pixels", |d: &mut DebugTerrain, v| d.show_pixels = v),
                                                debug_switch("Show UV", |d: &mut DebugTerrain, v| d.show_uv = v),
                                                debug_switch("Show Normals", |d: &mut DebugTerrain, v| d.show_normals = v),
                                                //debug_switch("Show Surface", |d: &mut DebugTerrain, v| d.show_surface = v),
                                                debug_switch_checked("Morph", |d: &mut DebugTerrain, v| d.morph = v),
                                                debug_switch_checked("Blend", |d: &mut DebugTerrain, v| d.blend = v),
                                                debug_switch("Tile Tree LOD", |d: &mut DebugTerrain, v| d.tile_tree_lod = v),
                                                debug_switch("Freeze Frustum", |d: &mut DebugTerrain, v| d.freeze = v),
                                            ]
                                        ),
                                    ],
                                ],
                            ],
                            subpane() Children [
                                subpane_header() Children [
                                    (Text("Earth") ThemedText),
                                ],
                                subpane_body() Children [
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<OceanColorGroupBody>(),
                                            (Text("Ocean Colors") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            OceanColorGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                material_slider("Transition Band (m)", 0.0, 100.0, 0.5, 5.0, |c, v| c.ocean_transition_band = v),
                                                color_field("Deep Color", Vec3::new(0.0, 0.0, 0.0), |c| c.ocean_deep_color,
                                                    |c, v| c.ocean_deep_color.x = v, |c, v| c.ocean_deep_color.y = v, |c, v| c.ocean_deep_color.z = v),
                                                color_field("Shallow Color", Vec3::new(0.0264, 0.0864, 0.216), |c| c.ocean_shallow_color,
                                                    |c, v| c.ocean_shallow_color.x = v, |c, v| c.ocean_shallow_color.y = v, |c, v| c.ocean_shallow_color.z = v),
                                                color_field("Specular Color", Vec3::new(1.0, 1.0, 1.0), |c| c.ocean_specular_color,
                                                    |c, v| c.ocean_specular_color.x = v, |c, v| c.ocean_specular_color.y = v, |c, v| c.ocean_specular_color.z = v),
                                                color_field("Fresnel Color", Vec3::new(0.08, 0.18, 0.35), |c| c.ocean_fresnel_color,
                                                    |c, v| c.ocean_fresnel_color.x = v, |c, v| c.ocean_fresnel_color.y = v, |c, v| c.ocean_fresnel_color.z = v),
                                                color_field("Ambient", Vec3::new(0.0, 0.0, 0.0), |c| c.ocean_ambient,
                                                    |c, v| c.ocean_ambient.x = v, |c, v| c.ocean_ambient.y = v, |c, v| c.ocean_ambient.z = v),
                                                color_field("Chlorophyll Color", Vec3::new(0.055, 0.150, 0.095), |c| c.ocean_chlorophyll_color,
                                                    |c, v| c.ocean_chlorophyll_color.x = v, |c, v| c.ocean_chlorophyll_color.y = v, |c, v| c.ocean_chlorophyll_color.z = v),
                                                color_field("Polar Color", Vec3::new(0.010, 0.045, 0.140), |c| c.ocean_polar_color,
                                                    |c, v| c.ocean_polar_color.x = v, |c, v| c.ocean_polar_color.y = v, |c, v| c.ocean_polar_color.z = v),
                                                color_field("Equatorial Color", Vec3::new(0.060, 0.220, 0.360), |c| c.ocean_equatorial_color,
                                                    |c, v| c.ocean_equatorial_color.x = v, |c, v| c.ocean_equatorial_color.y = v, |c, v| c.ocean_equatorial_color.z = v),
                                                material_slider("Bathymetry Strength", 0.0, 4.0, 0.05, 1.2, |c, v| c.bathymetry_strength = v),
                                                material_slider("Chlorophyll Strength", 0.0, 15.0, 0.1, 3.0, |c, v| c.chlorophyll_strength = v),
                                                material_slider("Latitude Tint Strength", 0.0, 1.0, 0.01, 0.4, |c, v| c.ocean_latitude_strength = v),
                                            ]
                                        ),
                                    ],
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<WaveGroupBody>(),
                                            (Text("Wave Normal") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            WaveGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                material_slider("Scale A", 0.0, 30.0, 0.1, 8.0, |c, v| c.wave_scale_a = v),
                                                material_slider("Scale B", 0.0, 30.0, 0.1, 13.0, |c, v| c.wave_scale_b = v),
                                                material_slider("Strength", 0.0, 2.0, 0.01, 0.50, |c, v| c.wave_strength = v),
                                                material_slider("Speed A", 0.0, 0.5, 0.001, 0.010, |c, v| c.wave_speed_a = v),
                                                material_slider("Speed B", 0.0, 0.5, 0.001, 0.07, |c, v| c.wave_speed_b = v),
                                                material_slider("Distance Ref", 0.0, 2.0e7, 1.0e5, 5.0e6, |c, v| c.wave_dist_ref = v),
                                            ]
                                        ),
                                    ],
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<SpecularGroupBody>(),
                                            (Text("Ocean") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            SpecularGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                material_slider("Specular Smoothness", 0.001, 0.5, 0.001, 0.030, |c, v| c.spec_smoothness = v),
                                                material_slider("Fresnel Power", 0.0, 10.0, 0.1, 4.5, |c, v| c.fresnel_power = v),
                                                material_slider("Fresnel Weight", 0.0, 1.0, 0.01, 0.45, |c, v| c.fresnel_weight = v),
                                            ]
                                        ),
                                    ],
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<ShoreFoamGroupBody>(),
                                            (Text("Shore Foam") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            ShoreFoamGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                material_slider("Ripple Freq", 0.0, 200.0, 1.0, 42.0, |c, v| c.shore_ripple_freq = v),
                                                material_slider("Ripple Speed", 0.0, 10.0, 0.1, 2.5, |c, v| c.shore_ripple_speed = v),
                                                material_slider("Foam Falloff", 0.0, 30.0, 0.1, 8.5, |c, v| c.shore_foam_falloff = v),
                                                material_slider("Foam Strength", 0.0, 2.0, 0.01, 0.55, |c, v| c.shore_foam_strength = v),
                                                material_slider("Foam Width", 0.0, 1.0, 0.01, 0.12, |c, v| c.shore_foam_width = v),
                                                material_slider("Noise Scale", 0.0, 1000.0, 1.0, 400.0, |c, v| c.shore_noise_scale = v),
                                                material_slider("Noise Span", 0.0, 50.0, 0.1, 12.5, |c, v| c.shore_noise_span = v),
                                            ]
                                        ),
                                    ],
                                    group() Children [
                                        group_header() Children [
                                            disclosure_toggle::<AtmosphereGroupBody>(),
                                            (Text("Atmosphere") ThemedText),
                                        ],
                                        (
                                            group_body()
                                            AtmosphereGroupBody
                                            Node { display: Display::None }
                                            Children [
                                                // EarthAtmosphereSettings::default() values (atmosphere/mod.rs) - the
                                                // rest of that struct's fields are recomputed every frame from the
                                                // camera/light (see atmosphere/systems.rs) so aren't exposed here.
                                                atmosphere_slider("Radius Scale", 1.0, 1.5, 0.005, 1.125, |s, v| s.atmosphere_radius_scale = v),
                                                atmosphere_slider("Ambient Scatter Strength", 0.0, 20.0, 0.1, 5.0, |s, v| s.ambient_scatter_strength = v),
                                                atmosphere_slider("Cloud Color R", 0.0, 1.0, 0.01, 1.0, |s, v| s.cloud_color.x = v),
                                                atmosphere_slider("Cloud Color G", 0.0, 1.0, 0.01, 1.0, |s, v| s.cloud_color.y = v),
                                                atmosphere_slider("Cloud Color B", 0.0, 1.0, 0.01, 1.0, |s, v| s.cloud_color.z = v),
                                                atmosphere_slider("Cloud Coverage", 0.0, 1.0, 0.01, 0.55, |s, v| s.cloud_coverage = v),
                                                atmosphere_slider("Cloud Altitude Scale", 1.0, 1.05, 0.0005, 1.006, |s, v| s.cloud_altitude_scale = v),
                                                atmosphere_slider("Cloud Noise Scale", 0.5, 15.0, 0.1, 3.0, |s, v| s.cloud_scale = v),
                                                atmosphere_slider("Cloud Wind Speed", 0.0, 0.2, 0.001, 0.015, |s, v| s.cloud_speed = v),
                                                atmosphere_slider("Cloud Sharpness", 0.1, 5.0, 0.05, 1.5, |s, v| s.cloud_density = v),
                                            ]
                                        ),
                                    ],
                                ],
                            ]
                        ]
                    ),
                ]
            ),
        ]
    }
}
