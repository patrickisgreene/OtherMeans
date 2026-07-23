use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
pub struct Movement {
    #[serde(default)]
    pub range: MovementRange,
    pub speed: u32,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
pub enum MovementRange {
    #[default]
    Unlimited,
    Limited(u32),
}
