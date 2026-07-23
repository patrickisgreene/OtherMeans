use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
pub enum Domain {
    Sea,
    Air,
    Bomb,
    Land,
    Space,
    Cyber,
    Missile,
    Intelligence,
}
