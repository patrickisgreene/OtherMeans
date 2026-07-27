use bevy::prelude::*;
use cities::CityScaleRank;
use terrain::{math::TerrainShape, view::TerrainViewConfig};

fn on_screen(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    transform: &GlobalTransform,
) -> bool {
    let Ok(viewport_position) = camera.world_to_viewport(camera_transform, transform.translation())
    else {
        return false;
    };
    let Some(size) = camera.logical_viewport_size() else {
        return false;
    };
    viewport_position.cmpge(Vec2::ZERO).all() && viewport_position.cmple(size).all()
}

pub fn visible(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    view_config: Option<&TerrainViewConfig>,
    occluder_pos: Option<Vec3>,
    transform: &GlobalTransform,
    lod: Option<&CityScaleRank>,
) -> bool {
    if !on_screen(camera, camera_transform, transform) {
        return false;
    }

    let camera_pos = camera_transform.translation();
    let label_pos = transform.translation();

    if let Some(occluder_pos) = occluder_pos {
        let normal = (label_pos - occluder_pos).normalize_or_zero();
        if normal.dot(camera_pos - label_pos) <= 0.0 {
            return false;
        }
    }

    if let (Some(view_config), Some(lod)) = (view_config, lod) {
        let lod_distance = (view_config.blend_distance * TerrainShape::WGS84.face_size()) as f32;
        let level = (lod_distance / camera_pos.distance(label_pos))
            .log2()
            .floor()
            .max(0.0);
        if f32::from(lod.0) > level {
            return false;
        }
    }

    true
}
