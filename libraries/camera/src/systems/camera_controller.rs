use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    math::{DMat3, DQuat, DVec3},
    prelude::*,
    window::PrimaryWindow,
};
use big_space::prelude::*;

use crate::EarthCameraController;

pub fn earth_camera_controller(
    time: Res<Time<Real>>,
    grids: Grids,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
        &mut Transform,
        &mut CellCoord,
        &mut EarthCameraController,
    )>,
) {
    let Ok((cam_entity, cam, cam_global, mut transform, mut cell_coord, mut controller)) =
        camera.single_mut()
    else {
        return;
    };

    if !controller.enabled {
        return;
    }

    // ── Zoom ────────────────────────────────────────────────────────────────
    let min_distance = controller.radius * 1.025;
    let max_distance = controller.max_distance;
    for event in mouse_wheel.read() {
        let speed = match event.unit {
            MouseScrollUnit::Line => 0.5,
            MouseScrollUnit::Pixel => 0.0005,
        };
        let log = controller.target_distance.log2() - event.y as f64 * speed;
        controller.target_distance =
            2.0_f64.powf(log.clamp(min_distance.log2(), max_distance.log2()));
    }

    if (controller.target_distance - controller.distance).abs() > 1e-3 {
        let smoothing = (time.delta_secs_f64() / 0.1).min(1.0);
        controller.distance += (controller.target_distance - controller.distance) * smoothing;
    }

    // ── Anchor-based pan ─────────────────────────────────────────────────────
    let Ok(window) = windows.single() else { return };

    if mouse_buttons.just_pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Left) {
        let up_dir = DVec3::new(
            controller.latitude.cos() * controller.longitude.sin(),
            controller.latitude.sin(),
            controller.latitude.cos() * controller.longitude.cos(),
        );
        let camera_pos = up_dir * controller.distance;
        let radius = controller.radius;

        let cursor_hit = |cursor: Vec2| -> Option<DVec3> {
            let ray = cam.viewport_to_world(cam_global, cursor).ok()?;
            ray_sphere_intersection(camera_pos, ray.direction.as_dvec3(), radius)
                .map(|hit| hit.normalize())
        };

        if mouse_buttons.just_pressed(MouseButton::Left) {
            controller.anchor = window.cursor_position().and_then(cursor_hit);
        }

        if mouse_buttons.pressed(MouseButton::Left) {
            if let Some(anchor_dir) = controller.anchor {
                if let Some(current_dir) = window.cursor_position().and_then(cursor_hit) {
                    let dot = current_dir.dot(anchor_dir).clamp(-1.0, 1.0);
                    if (1.0 - dot).abs() > 1e-10 && (1.0 + dot).abs() > 1e-10 {
                        let rotation = DQuat::from_rotation_arc(current_dir, anchor_dir);
                        let new_up = (rotation * up_dir).normalize();
                        controller.latitude = new_up.y.asin();
                        controller.longitude = new_up.x.atan2(new_up.z);
                        controller.latitude = controller.latitude.clamp(-1.54, 1.54);
                    }
                }
            }
        }
    } else if controller.anchor.is_some() {
        controller.anchor = None;
    }

    // ── Rebuild position & orientation from (possibly updated) lat/lon ───────
    let cos_lat = controller.latitude.cos();
    let sin_lat = controller.latitude.sin();
    let cos_lon = controller.longitude.cos();
    let sin_lon = controller.longitude.sin();

    let up = DVec3::new(cos_lat * sin_lon, sin_lat, cos_lat * cos_lon);
    let east = DVec3::Y.cross(up).normalize();
    let north = up.cross(east).normalize();

    let camera_position = up * controller.distance;
    let new_rotation = DQuat::from_mat3(&DMat3::from_cols(east, north, up)).as_quat();

    // Split the large world-space position across the big_space grid cell and
    // the local f32 Transform so floating-point precision is preserved.
    if let Some(grid) = grids.parent_grid(cam_entity) {
        let (new_cell, new_translation) = grid.translation_to_grid(camera_position);
        if *cell_coord != new_cell {
            *cell_coord = new_cell;
        }
        if transform.translation != new_translation {
            transform.translation = new_translation;
        }
    } else {
        let new_translation = camera_position.as_vec3();
        if transform.translation != new_translation {
            transform.translation = new_translation;
        }
    }
    if transform.rotation != new_rotation {
        transform.rotation = new_rotation;
    }
}

fn ray_sphere_intersection(origin: DVec3, direction: DVec3, radius: f64) -> Option<DVec3> {
    // Sphere centred at the world origin.
    let b = 2.0 * origin.dot(direction);
    let c = origin.dot(origin) - radius * radius;
    let discriminant = b * b - 4.0 * c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / 2.0;
    let t2 = (-b + sqrt_d) / 2.0;
    [t1, t2]
        .into_iter()
        .filter(|&t| t >= 0.0)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .map(|t| origin + direction * t)
}
