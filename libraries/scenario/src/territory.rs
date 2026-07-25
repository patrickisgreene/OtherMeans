use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ControlledTerritory {
    Place(u32),
    Country([char; 3]),
}
