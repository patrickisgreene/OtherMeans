use bevy::prelude::*;

use scenario::Scenario;

#[derive(Resource)]
pub struct GameScenario(pub Handle<Scenario>);

#[derive(Resource)]
pub struct EarthConfig(pub Handle<terrain::config::TerrainConfig>);

#[derive(Resource)]
pub struct GameCitiesDatabase(pub Handle<cities::descriptor::CitiesDatabase>);
