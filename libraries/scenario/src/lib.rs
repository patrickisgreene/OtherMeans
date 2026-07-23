mod combatant;
mod scenario;
mod territory;
mod weapons;

pub use combatant::*;
pub use scenario::*;
pub use territory::*;
pub use weapons::*;

pub type WeaponCount = usize;
pub type StatisticModifier = u8;
pub type PlaceId = u32;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

#[derive(Resource)]
pub struct GameScenario(pub Handle<Scenario>);

#[derive(Resource)]
pub struct EarthConfig(pub Handle<terrain::config::TerrainConfig>);

#[derive(Resource)]
pub struct GameCitiesDatabase(pub Handle<cities::descriptor::CitiesDatabase>);

pub struct ScenarioPlugin;

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<Scenario>::new(&["scenario.ron"]));
    }
}
