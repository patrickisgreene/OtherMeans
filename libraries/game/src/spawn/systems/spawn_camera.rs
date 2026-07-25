use bevy::prelude::*;
use camera::EarthCameraController;
use workspace::EARTH_RADIUS;

use crate::GameCamera;

pub fn spawn_camera(mut commands: Commands, query: Query<Entity, With<GameCamera>>) {
    commands.entity(query.single().unwrap()).insert((
        Transform::from_translation(-Vec3::X * EARTH_RADIUS as f32 * 3.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        EarthCameraController::default(),
    ));
}
