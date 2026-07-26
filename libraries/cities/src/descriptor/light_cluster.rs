use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CityLightCluster {
    pub lat: f64,
    pub lon: f64,
    pub population: u32,
    pub city_count: u16,
}

#[derive(Asset, TypePath, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CityLightsDatabase(pub Vec<CityLightCluster>);
