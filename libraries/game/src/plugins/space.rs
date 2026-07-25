use bevy::prelude::*;
use big_space::prelude::*;

use crate::{Earth, GameCamera, GameState};

pub struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), spawn_space);
    }
}

fn spawn_space(mut commands: Commands) {
    commands.spawn_big_space(Grid::default(), |grid| {
        grid.insert(InheritedVisibility::default());
        grid.spawn_spatial(GameCamera);

        grid.spawn_spatial(Earth);
    });
}
