use bevy::prelude::*;

use super::Formation;
use crate::data::Weapon;

#[derive(Debug, PartialEq, Clone)]
pub struct StrikePackage {
    pub count: u32,
    pub weapon: Handle<Weapon>,
    pub formation: Formation,
}
