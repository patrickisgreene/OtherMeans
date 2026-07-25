use bevy::prelude::*;
use big_space::prelude::*;
use earth::EarthAtmosphereSettings;
use terrain::view::TerrainViewConfig;

#[derive(Component)]
#[require(Name::new("Earth"))]
#[require(InheritedVisibility::default())]
pub struct Earth;

#[derive(Component)]
#[require(Name::new("GameCamera"))]
#[require(TerrainViewConfig::default())]
#[require(FloatingOrigin = FloatingOrigin)]
#[require(EarthAtmosphereSettings::default())]
pub struct GameCamera;
