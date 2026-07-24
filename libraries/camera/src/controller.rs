use bevy::{math::DVec3, prelude::*};

#[derive(Clone, Debug, Component)]
#[require(Camera3d)]
pub struct EarthCameraController {
    pub enabled: bool,

    /// Radius of the earth sphere in metres.
    pub radius: f64,
    /// Longitude in radians.
    pub longitude: f64,
    /// Latitude in radians, clamped to ±π/2.
    pub latitude: f64,
    /// Radial distance from the Earth's centre in metres.
    pub distance: f64,

    /// Mouse-drag sensitivity (radians per pixel), used only when the cursor misses the sphere.
    pub sensitivity: f64,

    /// Maximum allowed distance, kept in sync by `update_camera_max_distance`.
    pub max_distance: f64,

    // Internal: smoothed zoom target.
    pub target_distance: f64,
    // World-space unit vector from Earth's centre to the drag anchor point.
    pub(crate) anchor: Option<DVec3>,
}

impl Default for EarthCameraController {
    fn default() -> Self {
        let radius = 6371000.0;
        // Placeholder — overwritten on first frame by update_camera_max_distance.
        let max_distance = radius * 1.3;
        Self {
            enabled: true,
            radius,
            longitude: 0.0,
            latitude: 0.0,
            distance: max_distance,
            max_distance,
            sensitivity: 0.005,
            target_distance: max_distance,
            anchor: None,
        }
    }
}
