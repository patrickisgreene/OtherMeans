use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::CityDescriptor;

#[derive(Asset, TypePath, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CitiesDatabase(pub Vec<CityDescriptor>);
