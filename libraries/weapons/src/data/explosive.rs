use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
pub enum Explosive {
    High,
    Nuclear,
    SubMunition,
}
