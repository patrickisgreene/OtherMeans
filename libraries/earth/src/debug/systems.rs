use bevy::{
    feathers::controls::{ColorPlaneValue, ColorSwatchValue, FeathersDisclosureToggle},
    prelude::*,
    ui_widgets::{ValueChange, checkbox_self_update},
};

use crate::{
    EarthMaterial,
    debug::{ColorPlaneOf, ColorPreviewOf, DebugPanel},
};

pub fn toggle_render_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<EarthMaterial>>,
) {
    if input.just_pressed(KeyCode::F11) {
        for material in materials.iter_mut() {
            material.1.render_mode += 1;
            if material.1.render_mode == 7 {
                material.1.render_mode = 0;
            }
        }
    }
}

/// A chevron toggle that shows/hides every entity marked `T` (there's normally exactly one per
/// toggle - each of [`LodGroupBody`]/[`TerrainGroupBody`]/[`RenderingGroupBody`] is only used
/// once in [`debug_panel`]) by flipping its `Node.display` between `Flex` and `None`. Starts
/// unchecked (collapsed, pointing right); [`collapse_settings_groups`] hides the bodies to
/// match on startup, since a plain `Node` override can't be layered onto `group_body()`'s
/// scene-inherited styling to start it hidden directly.
pub fn disclosure_toggle<T: Component>() -> impl Scene {
    bsn! {
        (
            @FeathersDisclosureToggle
            on(checkbox_self_update)
            on(|change: On<ValueChange<bool>>, mut bodies: Query<&mut Node, With<T>>| {
                for mut node in bodies.iter_mut() {
                    node.display = if change.value { Display::Flex } else { Display::None };
                }
            })
        )
    }
}

/// Keeps every [`color_field`] swatch and [`FeathersColorPlane`] showing the live value of the
/// `EarthConstants` field it previews (there's normally exactly one live [`TerrainMaterial`] -
/// see `initialize()`). The plane only ever reports its own red/blue axes back out, so it needs
/// this to pick up green-slider edits (and to stay put if it's ever driven from elsewhere) the
/// same way the swatch always has.
pub fn sync_color_swatches(
    materials: Res<Assets<EarthMaterial>>,
    mut swatches: Query<(&ColorPreviewOf, &mut ColorSwatchValue)>,
    mut planes: Query<(&ColorPlaneOf, &mut ColorPlaneValue)>,
) {
    let Some((_, material)) = materials.iter().next() else {
        return;
    };
    for (preview, mut swatch) in swatches.iter_mut() {
        let v = (preview.0)(&material.constants).clamp(Vec3::ZERO, Vec3::ONE);
        swatch.0 = Color::srgb(v.x, v.y, v.z);
    }
    for (plane_of, mut plane) in planes.iter_mut() {
        let v = (plane_of.0)(&material.constants).clamp(Vec3::ZERO, Vec3::ONE);
        // FeathersColorPlane::RedBlue: x/y are the plane's two axes (red/blue), z is the fixed
        // channel driving its background gradient (green) - same mapping bevy's own gallery
        // example uses for its Red/Blue plane.
        plane.0 = Vec3::new(v.x, v.z, v.y);
    }
}

pub fn toggle_panel(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, With<DebugPanel>>,
) {
    if input.just_pressed(KeyCode::F12) {
        if let Ok(entity) = query.single() {
            commands.entity(entity).despawn();
        } else {
            commands.spawn_scene(super::debug_panel());
        }
    }
}
