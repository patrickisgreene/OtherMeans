use bevy::{dev_tools::fps_overlay::FpsOverlayPlugin, prelude::*};

use game::{GameCitiesDatabase, GamePlugins, GameScenario, GameState};
use scenario::Scenario;
use workspace::default_plugins_big_space;

fn main() {
    App::new()
        .add_plugins((default_plugins_big_space(None), GamePlugins))
        .add_plugins(FpsOverlayPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Startup, start.after(setup))
        .run();
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut scenarios: ResMut<Assets<Scenario>>,
) {
    commands.insert_resource(GameScenario(scenarios.add(Scenario::players_america())));
    commands.insert_resource(GameCitiesDatabase(assets.load("earth/cities.ron")));
}

fn start(mut state: ResMut<NextState<GameState>>) {
    state.set(GameState::Loading);
}
