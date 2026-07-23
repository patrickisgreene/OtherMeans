use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ControlledTerritory {
    Place(usize),
    Country([char; 3]),
}
