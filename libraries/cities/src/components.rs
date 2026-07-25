use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default)]
pub struct CityLabelRank(pub u8);

#[derive(Component, Clone, Copy, Default)]
pub struct CityScaleRank(pub u8);

#[derive(Component, Default, Clone, Copy)]
pub struct CitiesRoot;
