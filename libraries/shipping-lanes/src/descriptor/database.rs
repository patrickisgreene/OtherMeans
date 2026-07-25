use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::ShippingLaneDescriptor;

#[derive(Asset, TypePath, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShippingLaneNetwork(pub Vec<ShippingLaneDescriptor>);
