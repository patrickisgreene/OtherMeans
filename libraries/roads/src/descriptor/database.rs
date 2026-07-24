use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::RoadDescriptor;

#[derive(Asset, TypePath, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct RoadNetwork(pub Vec<RoadDescriptor>);
