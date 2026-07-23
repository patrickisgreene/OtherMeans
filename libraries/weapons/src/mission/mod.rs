mod operation;
mod package;
mod state;
mod waypoint;

pub use operation::*;
pub use package::*;
pub use state::*;
pub use waypoint::*;

use bevy::prelude::*;

#[derive(Debug, Default, PartialEq, Clone, Copy, Hash, Eq)]
pub enum FireType {
    #[default]
    Direct,
    Indirect,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Hash, Eq)]
pub enum Formation {
    #[default]
    Echelon,
    Diamond,
}

#[derive(Component, Debug, PartialEq, Clone)]
#[require(StrikeState::default())]
pub struct StrikeMission {
    pub package: StrikePackage,
    pub waypoints: Vec<Waypoint>,
}
