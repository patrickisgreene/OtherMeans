use bevy::prelude::*;

#[derive(Component, Default, Debug, PartialEq, Clone, Copy)]
pub struct StrikeState {
    /// Current speed in m/s. Falls back to the weapon asset's speed if zero.
    pub speed: f32,
    /// Index of the waypoint the package is currently travelling *from*.
    pub progress: usize,
    /// Normalised progress [0, 1] within the current waypoint segment.
    pub segment_t: f32,
}
