use bevy::prelude::*;
use cities::descriptor::CitiesDatabase;

use crate::{GameCitiesDatabase, GameScenario, GameState};
use scenario::Scenario;

pub fn load_status(
    scenario: Res<GameScenario>,
    scenarios: Res<Assets<Scenario>>,
    city_db: Res<GameCitiesDatabase>,
    cities: Res<Assets<CitiesDatabase>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if scenarios.get(&scenario.0).is_none() {
        return;
    }

    if cities.get(&city_db.0).is_none() {
        return;
    }
    state.set(GameState::Spawning);
}
