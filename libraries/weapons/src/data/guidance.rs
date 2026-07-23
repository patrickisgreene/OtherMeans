use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
pub enum Guidance {
    Gps,
    Dumb,
    Human,
    Radar,
    Laser,
    Infrared,
    Inertial,
}
