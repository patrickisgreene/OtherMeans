use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default)]
pub struct CityLightsRoot;

#[derive(Component, Clone, Copy)]
pub struct CityLightCluster {
    pub population: u32,
    pub city_count: u16,
}
