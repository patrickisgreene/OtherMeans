mod domain;
mod explosive;
mod guidance;
mod movement;
mod payload;

pub use domain::*;
pub use explosive::*;
pub use guidance::*;
pub use movement::*;
pub use payload::*;

use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Eq, Asset, TypePath, Serialize, Deserialize)]
pub struct Weapon {
    pub id: String,
    pub name: String,
    pub domain: Domain,
    pub guidance: Guidance,
    pub movement: Movement,
    pub payload: Payload,
    pub attributes: HashMap<String, ron::Value>,
}
