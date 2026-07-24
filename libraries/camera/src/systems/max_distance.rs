use bevy::{
    camera::Projection,
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};

use crate::EarthCameraController;

/// Recomputes `max_distance` so the sphere always fills the entire viewport.
///
/// The sphere's angular radius must reach the screen corners, so:
///   max_distance = radius / sin(atan(tan(fov_v/2) * sqrt(aspect² + 1)))
///
/// Runs every frame but skips when the window size has not changed.
pub fn update_camera_max_distance(
    mut controllers: Query<(&mut EarthCameraController, &Projection)>,
    mut resize_events: MessageReader<WindowResized>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if resize_events.read().next().is_none() {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let w = window.physical_width();
    let h = window.physical_height();
    if h == 0 {
        return;
    }

    let aspect = w as f64 / h as f64;

    for (mut controller, projection) in &mut controllers {
        let Projection::Perspective(persp) = projection else {
            continue;
        };
        let fov_v = persp.fov as f64;
        let half_v_tan = (fov_v / 2.0).tan();
        let k = half_v_tan * (aspect * aspect + 1.0).sqrt();
        let max_distance = controller.radius * (1.0 + k * k).sqrt() / k;

        controller.max_distance = max_distance;
        controller.target_distance = controller.target_distance.min(max_distance);
        controller.distance = controller.distance.min(max_distance);
    }
}
