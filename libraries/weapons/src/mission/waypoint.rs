use super::{FireType, Operation};

#[derive(Debug, PartialEq, Clone)]
pub struct Waypoint {
    pub lat: f64,
    pub long: f64,
    pub fire_type: FireType,
    pub strike: Option<Operation>,
}
