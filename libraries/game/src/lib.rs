use bevy::{app::PluginGroupBuilder, prelude::*};

mod components;
mod plugins;
mod resources;
mod spawn;

pub use components::*;
pub use resources::*;
pub use spawn::*;

use crate::plugins::InterfacePlugin;

#[derive(States, Default, Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub enum GameState {
    #[default]
    Uninitialized,
    Loading,
    Running,
    Spawning,
}

pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(plugins::StatePlugin)
            .add(SpawnPlugin)
            .add(plugins::SpacePlugin)
            .add(camera::EarthCameraPlugin)
            .add(earth::EarthPlugin)
            .add(cities::CitiesPlugin)
            .add(labels::LabelsPlugin::new(GameState::Spawning))
            .add(roads::RoadsPlugin)
            .add(buildings::BuildingsPlugin)
            .add(vehicles::VehiclesPlugin)
            .add(scenario::ScenarioPlugin)
            .add(shipping_lanes::ShippingLanesPlugin)
            .add(shipping::ShippingPlugin)
            .add(InterfacePlugin)
            .add_group(bevy::feathers::FeathersPlugins)
    }
}
