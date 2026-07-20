use bevy::prelude::*;

use crate::EarthParams;

#[derive(Component, Default, Clone)]
pub struct DebugPanel;

/// Marker for the "Level Of Detail" group's body, used by its disclosure toggle (see
/// [`disclosure_toggle`]) to find and show/hide it.
#[derive(Component, Default, Clone)]
pub struct LodGroupBody;

/// Marker for the "Terrain" group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct TerrainGroupBody;

/// Marker for the "Rendering" group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct RenderingGroupBody;

/// Marker for the "Ocean Colors" constants group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct OceanColorGroupBody;

/// Marker for the "Wave Normal" constants group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct WaveGroupBody;

/// Marker for the "Specular" constants group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct SpecularGroupBody;

/// Marker for the "Fresnel" constants group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct FresnelGroupBody;

/// Marker for the "Shore Foam" constants group's body; see [`LodGroupBody`].
#[derive(Component, Default, Clone)]
pub struct ShoreFoamGroupBody;

/// Marker pairing a [`FeathersColorSwatch`] spawned by [`color_field`] with the `EarthConstants`
/// field it previews, so [`sync_color_swatches`] can keep the swatch's color live as the field
/// is edited via the menu's R/G/B sliders.
#[derive(Component, Clone, Copy)]
pub struct ColorPreviewOf(pub fn(&EarthParams) -> Vec3);

impl Default for ColorPreviewOf {
    // bsn! requires every component type to implement Default even when a scene always provides
    // an explicit value (see color_field) - never actually read since color_field always
    // overrides it with a real field getter.
    fn default() -> Self {
        Self(|_| Vec3::ZERO)
    }
}

/// Marker pairing a [`FeathersColorPlane`] spawned by [`color_field`] with the `EarthConstants`
/// field it previews, mirroring [`ColorPreviewOf`]. The plane only reports pointer input for its
/// own two axes (red/blue) - without this, dragging the field's separate green slider (or the
/// red/blue sliders) would leave the plane's thumb position and background gradient stale, since
/// nothing would tell it the field changed. [`sync_color_swatches`] keeps all three live instead.
#[derive(Component, Clone, Copy)]
pub struct ColorPlaneOf(pub fn(&EarthParams) -> Vec3);

impl Default for ColorPlaneOf {
    // See ColorPreviewOf::default - same reasoning, never actually read.
    fn default() -> Self {
        Self(|_| Vec3::ZERO)
    }
}
