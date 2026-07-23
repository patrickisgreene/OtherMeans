use bevy::platform::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::{CombatantId, PlaceId, WeaponCount};

#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
pub enum WeaponsPlacementStrategy {
    Empty,
    #[default]
    Default,
    Random(HashMap<CombatantId, HashMap<String, WeaponCount>>),
    Static(HashMap<CombatantId, HashMap<PlaceId, HashMap<String, WeaponCount>>>),
}
