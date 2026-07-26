use bevy::prelude::*;
use terrain::TerrainConfigHandle;

use crate::lighting::CityLightCluster;

pub fn update_city_light_visibility(
    camera: Query<&GlobalTransform, With<Camera>>,
    sun: Query<&GlobalTransform, With<DirectionalLight>>,
    occluder: Query<&GlobalTransform, With<TerrainConfigHandle>>,
    mut clusters: Query<(&GlobalTransform, &mut Visibility), With<CityLightCluster>>,
) {
    let (Ok(camera_transform), Ok(occluder_transform)) = (camera.single(), occluder.single())
    else {
        return;
    };

    let camera_pos = camera_transform.translation();
    let occluder_pos = occluder_transform.translation();

    // Mirrors earth::atmosphere::systems' fallback sun direction, used until a real
    // DirectionalLight exists in the scene.
    let sun_dir = sun
        .single()
        .ok()
        .map(|transform| transform.forward().as_vec3())
        .unwrap_or(Vec3::new(1.0, 0.3, 0.5).normalize());

    for (transform, mut visibility) in &mut clusters {
        let pos = transform.translation();
        let normal = (pos - occluder_pos).normalize_or_zero();

        let facing_camera = normal.dot(camera_pos - pos) > 0.0;
        let night_side = normal.dot(sun_dir) > 0.0;

        *visibility = if facing_camera && night_side {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
