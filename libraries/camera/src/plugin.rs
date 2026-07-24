use bevy::prelude::*;

use crate::systems;

pub struct EarthCameraPlugin;

impl Plugin for EarthCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::update_camera_max_distance,
                systems::earth_camera_controller,
            )
                .chain(),
        );
    }
}
