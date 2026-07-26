use bevy::prelude::*;

use crate::GameState;

use super::systems;

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            systems::load_status.run_if(in_state(GameState::Loading)),
        )
        .add_systems(
            OnEnter(GameState::Spawning),
            (
                systems::spawn_camera,
                systems::spawn_earth,
                systems::spawn_cities,
                systems::spawn_city_lights,
                systems::spawn_combatants,
            ),
        );
    }
}
