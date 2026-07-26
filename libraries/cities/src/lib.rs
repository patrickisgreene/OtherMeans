pub mod descriptor;
pub mod lighting;
pub mod preprocess;

mod components;

pub use components::*;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use serde::{Deserialize, Serialize};

pub struct CitiesPlugin;

impl Plugin for CitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<descriptor::CitiesDatabase>::new(&[
            "cities.ron",
        ]));
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum CityFilter {
    MinPopulation(u32),
}
