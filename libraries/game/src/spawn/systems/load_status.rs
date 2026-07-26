use bevy::prelude::*;
use cities::descriptor::{CitiesDatabase, CityLightsDatabase};

use crate::{GameCitiesDatabase, GameCityLightsDatabase, GameScenario, GameState};
use scenario::Scenario;

pub fn load_status(
    scenario: Res<GameScenario>,
    scenarios: Res<Assets<Scenario>>,
    city_db: Res<GameCitiesDatabase>,
    cities: Res<Assets<CitiesDatabase>>,
    city_lights_db: Res<GameCityLightsDatabase>,
    city_lights: Res<Assets<CityLightsDatabase>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if scenarios.get(&scenario.0).is_none() {
        return;
    }

    if cities.get(&city_db.0).is_none() {
        return;
    }

    if city_lights.get(&city_lights_db.0).is_none() {
        return;
    }
    state.set(GameState::Spawning);
}
