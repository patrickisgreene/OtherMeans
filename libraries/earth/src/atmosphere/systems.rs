use bevy::camera::CameraProjection;
use bevy::math::DVec3;
use bevy::prelude::*;
use big_space::prelude::*;

use crate::EarthAtmosphereSettings;

// TODO: Replace this by reading the Earth Shape.
pub const EARTH_RADIUS: f64 = 6_371_000.0;

/// Updates the PostProcessSettings with the camera position using 64-bit precision from big_space.
pub fn update_post_process_settings(
    grids: Grids,
    time: Res<Time>,
    mut camera_query: Query<
        (
            Entity,
            &CellCoord,
            &Transform,
            &Projection,
            &mut EarthAtmosphereSettings,
        ),
        // TODO: this only works if there is only 1 camera.
        With<Camera>,
    >,
    light_query: Query<&Transform, With<DirectionalLight>>,
) {
    for (entity, cell, transform, projection, mut settings) in &mut camera_query {
        let Some(grid) = grids.parent_grid(entity) else {
            continue;
        };

        settings.time = time.elapsed_secs();

        // Get 64-bit precision camera position from big_space
        let camera_position_f64 = grid.grid_position_double(cell, transform);
        settings.set_camera_position(camera_position_f64);

        // Planet center is at origin in world space (DVec3::ZERO)
        settings.set_planet_center(DVec3::ZERO);

        // Planet scale is the diameter (2 * radius)
        settings.planet_scale = (EARTH_RADIUS * 2.0) as f32;

        // Derive sun direction from the directional light's transform
        let sun_dir = light_query
            .single()
            .ok()
            .map(|t| t.forward().as_vec3())
            .unwrap_or(Vec3::new(1.0, 0.3, 0.5).normalize());
        settings.sun_position = sun_dir * 150_000_000_000.0; // ~1 AU away

        // Update projection matrices
        let proj_mat = match projection {
            Projection::Perspective(p) => p.get_clip_from_view(),
            Projection::Orthographic(o) => o.get_clip_from_view(),
            Projection::Custom(c) => c.get_clip_from_view(),
        };
        settings.proj_mat = proj_mat;
        settings.inverse_proj = proj_mat.inverse();

        // Update view matrices from transform
        let view_mat = transform.to_matrix().inverse();
        settings.view_mat = view_mat;
        settings.inverse_view = transform.to_matrix();
    }
}
