use bevy::prelude::*;
use scenario::Scenario;

#[derive(Resource)]
pub struct GameScenario(pub Handle<Scenario>);

#[derive(Resource)]
pub struct GameCitiesDatabase(pub Handle<cities::descriptor::CitiesDatabase>);

#[derive(Resource)]
pub struct GameCityLightsDatabase(pub Handle<cities::descriptor::CityLightsDatabase>);
