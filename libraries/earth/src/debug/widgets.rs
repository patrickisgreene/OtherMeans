use bevy::feathers::{controls::*, display::*};
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{
    SliderPrecision, SliderStep, ValueChange, checkbox_self_update, slider_self_update,
};
use terrain::data::{TileAtlas, TileTree};
use terrain::debug::DebugTerrain;
use terrain::view::TerrainViewComponents;

use crate::debug::{ColorPlaneOf, ColorPreviewOf};
use crate::{EarthAtmosphereSettings, EarthMaterial, EarthParams};

/// A labeled slider that writes into every live [`TileTree`] (there's normally exactly one,
/// keyed by (terrain, view) - see `initialize()`) whenever it's dragged, via `setter`. Silently
/// does nothing before the terrain has finished loading, matching the same
/// `tile_trees.values_mut()` pattern already used by the keyboard debug controls in
/// `terrain::debug::update_view_parameter`.
pub fn lod_slider(
    label_text: &str,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    setter: impl Fn(&mut TileTree, f64) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween
        }
        Children [
            label_dim(label_text),
            (
                @FeathersSlider {
                    @min: min,
                    @max: max,
                    @value: value,
                }
                SliderStep(step)
                SliderPrecision(2)
                on(slider_self_update)
                on(move |change: On<ValueChange<f32>>, mut tile_trees: ResMut<TerrainViewComponents<TileTree>>| {
                    for tile_tree in tile_trees.values_mut() {
                        setter(tile_tree, change.value as f64);
                    }
                })
            )
        ]
    }
}

/// A labeled toggle switch that writes into every live [`TileTree`] whenever it's flipped, via
/// `setter` - same `tile_trees.values_mut()` pattern as [`lod_slider`], just for a `bool`
/// instead of a slider value.
pub fn lod_switch(
    label_text: &str,
    setter: impl Fn(&mut TileTree, bool) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(8),
        }
        Children [
            label_dim(label_text),
            (
                @FeathersToggleSwitch
                on(checkbox_self_update)
                on(move |change: On<ValueChange<bool>>, mut tile_trees: ResMut<TerrainViewComponents<TileTree>>| {
                    for tile_tree in tile_trees.values_mut() {
                        setter(tile_tree, change.value);
                    }
                })
            ),
        ]
    }
}

/// Same as [`lod_switch`], but starts checked - for a `TileTree` field whose live default (set
/// where the `TileTree` is constructed, not by `TerrainViewConfig::default()`) is already
/// "on". `bsn!`'s `{expr}` interpolation only accepts a bare identifier here, not a conditional
/// expression, so (as with [`debug_switch`]/[`debug_switch_checked`]) there's no single function
/// that can toggle whether `Checked` is present based on a runtime `bool`.
pub fn lod_switch_checked(
    label_text: &str,
    setter: impl Fn(&mut TileTree, bool) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(8),
        }
        Children [
            label_dim(label_text),
            (
                @FeathersToggleSwitch
                Checked
                on(checkbox_self_update)
                on(move |change: On<ValueChange<bool>>, mut tile_trees: ResMut<TerrainViewComponents<TileTree>>| {
                    for tile_tree in tile_trees.values_mut() {
                        setter(tile_tree, change.value);
                    }
                })
            ),
        ]
    }
}

/// A labeled slider that writes into every live [`TileAtlas`] (there's normally exactly one -
/// the "earth" entity spawned in `initialize()`) whenever it's dragged, via `setter`. Silently
/// does nothing before the terrain has finished loading, matching the same
/// `tile_atlases.iter_mut()` pattern already used by `terrain::debug::update_terrain_parameter`.
pub fn terrain_slider(
    label_text: &str,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    setter: impl Fn(&mut TileAtlas, f32) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween
        }
        Children [
            label_dim(label_text),
            (
                @FeathersSlider {
                    @min: min,
                    @max: max,
                    @value: value,
                }
                SliderStep(step)
                SliderPrecision(2)
                on(slider_self_update)
                on(move |change: On<ValueChange<f32>>, mut tile_atlases: Query<&mut TileAtlas>| {
                    for mut tile_atlas in tile_atlases.iter_mut() {
                        setter(&mut tile_atlas, change.value);
                    }
                })
            )
        ]
    }
}

/// A labeled slider that writes into every live [`EarthAtmosphereSettings`] (there's normally
/// exactly one - the camera spawned in `initialize()`) whenever it's dragged, via `setter`. Same
/// pattern as [`terrain_slider`], just targeting the camera's atmosphere post-process settings
/// component instead of a `TileAtlas`.
pub fn atmosphere_slider(
    label_text: &str,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    setter: impl Fn(&mut EarthAtmosphereSettings, f32) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween
        }
        Children [
            label_dim(label_text),
            (
                @FeathersSlider {
                    @min: min,
                    @max: max,
                    @value: value,
                }
                SliderStep(step)
                SliderPrecision(3)
                on(slider_self_update)
                on(move |change: On<ValueChange<f32>>, mut settings: Query<&mut EarthAtmosphereSettings>| {
                    for mut settings in settings.iter_mut() {
                        setter(&mut settings, change.value);
                    }
                })
            )
        ]
    }
}

/// A labeled slider that writes into every live [`TerrainMaterial`]'s [`EarthConstants`] (there's
/// normally exactly one - see `initialize()`) whenever it's dragged, via `setter`. Same pattern
/// as [`lod_slider`]/[`terrain_slider`], just targeting the material's uniform constants instead.
pub fn material_slider(
    label_text: &str,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    setter: impl Fn(&mut EarthParams, f32) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween
        }
        Children [
            label_dim(label_text),
            (
                @FeathersSlider {
                    @min: min,
                    @max: max,
                    @value: value,
                }
                SliderStep(step)
                SliderPrecision(3)
                on(slider_self_update)
                on(move |change: On<ValueChange<f32>>, mut materials: ResMut<Assets<EarthMaterial>>| {
                    for material in materials.iter_mut() {
                        setter(&mut material.1.constants, change.value);
                    }
                })
            )
        ]
    }
}

/// A labeled color tint field, collapsed behind a [`FeathersMenu`] popup instead of always-on
/// widgets - there are 11 `Vec3` tint fields across the constants groups, and showing all of
/// them unfolded would swamp the panel. The menu button shows a [`FeathersColorSwatch`]
/// previewing the field's current value (kept live by [`sync_color_swatches`] via `getter`);
/// opening it reveals the same [`FeathersColorPlane`] + [`FeathersColorSlider`] combo as bevy's
/// own Feathers gallery example's RGB picker - a Red/Blue plane for two channels plus a Green
/// slider for the third (the plane's fixed background-gradient channel), all three kept
/// mutually in sync by [`sync_color_swatches`] via [`ColorPlaneOf`]. Every widget writes into
/// every live [`TerrainMaterial`] via `set_r`/`set_g`/`set_b`.
pub fn color_field(
    label_text: &str,
    initial: Vec3,
    getter: fn(&EarthParams) -> Vec3,
    set_r: impl Fn(&mut EarthParams, f32) + Send + Sync + Clone + 'static,
    set_g: impl Fn(&mut EarthParams, f32) + Send + Sync + Clone + 'static,
    set_b: impl Fn(&mut EarthParams, f32) + Send + Sync + Clone + 'static,
) -> impl Scene {
    let initial_color = Color::srgb(
        initial.x.clamp(0.0, 1.0),
        initial.y.clamp(0.0, 1.0),
        initial.z.clamp(0.0, 1.0),
    );
    let plane_set_r = set_r.clone();
    let plane_set_b = set_b.clone();
    bsn! {
        @FeathersMenu
        Children [
            (
                @FeathersMenuButton {
                    @caption: bsn! {
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(6),
                        }
                        Children [
                            (
                                @FeathersColorSwatch
                                ColorSwatchValue(initial_color)
                                ColorPreviewOf(getter)
                            ),
                            label_dim(label_text),
                        ]
                    }
                }
            ),
            (
                @FeathersMenuPopup
                Node {
                    min_width: px(200),
                    row_gap: px(4),
                }
                Children [
                    (
                        @FeathersColorPlane::RedBlue
                        ColorPlaneValue({Vec3::new(initial.x, initial.z, initial.y)})
                        ColorPlaneOf(getter)
                        on(move |change: On<ValueChange<Vec2>>, mut materials: ResMut<Assets<EarthMaterial>>| {
                            for material in materials.iter_mut() {
                                plane_set_r(&mut material.1.constants, change.value.x);
                                plane_set_b(&mut material.1.constants, change.value.y);
                            }
                        })
                    ),
                    (
                        @FeathersColorSlider {
                            @value: {initial.x},
                            @channel: ColorChannel::Red
                        }
                        on(move |change: On<ValueChange<f32>>, mut materials: ResMut<Assets<EarthMaterial>>| {
                            for material in materials.iter_mut() {
                                set_r(&mut material.1.constants, change.value);
                            }
                        })
                    ),
                    (
                        @FeathersColorSlider {
                            @value: {initial.y},
                            @channel: ColorChannel::Green
                        }
                        on(move |change: On<ValueChange<f32>>, mut materials: ResMut<Assets<EarthMaterial>>| {
                            for material in materials.iter_mut() {
                                set_g(&mut material.1.constants, change.value);
                            }
                        })
                    ),
                    (
                        @FeathersColorSlider {
                            @value: {initial.z},
                            @channel: ColorChannel::Blue
                        }
                        on(move |change: On<ValueChange<f32>>, mut materials: ResMut<Assets<EarthMaterial>>| {
                            for material in materials.iter_mut() {
                                set_b(&mut material.1.constants, change.value);
                            }
                        })
                    ),
                ]
            )
        ]
    }
}

/// A labeled toggle switch, starting unchecked, that writes straight into the (always-live)
/// [`DebugTerrain`] resource via `setter`. For a switch that should start checked (matching a
/// `DebugTerrain::default()` field that's `true`), use [`debug_switch_checked`] instead - bsn!'s
/// `{expr}` interpolation only accepts a bare identifier at this position, not a conditional
/// expression, so there's no single function that can toggle whether `Checked` is present based
/// on a runtime `bool` argument.
pub fn debug_switch(
    label_text: &str,
    setter: impl Fn(&mut DebugTerrain, bool) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(8),
        }
        Children [
            label_dim(label_text),
            (
                @FeathersToggleSwitch
                on(checkbox_self_update)
                on(move |change: On<ValueChange<bool>>, mut debug: ResMut<DebugTerrain>| {
                    setter(&mut debug, change.value);
                })
            ),
        ]
    }
}

/// Same as [`debug_switch`], but starts checked - for fields that default to `true` in
/// `DebugTerrain::default()` (currently just `morph`/`blend`).
pub fn debug_switch_checked(
    label_text: &str,
    setter: impl Fn(&mut DebugTerrain, bool) + Send + Sync + Clone + 'static,
) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(8),
        }
        Children [
            label_dim(label_text),
            (
                @FeathersToggleSwitch
                Checked
                on(checkbox_self_update)
                on(move |change: On<ValueChange<bool>>, mut debug: ResMut<DebugTerrain>| {
                    setter(&mut debug, change.value);
                })
            ),
        ]
    }
}
