use super::StrikeMission;

#[derive(Debug, PartialEq, Clone)]
pub enum Operation {
    Refuel,
    Orbit(Option<usize>),
    Release(StrikeMission),
}
