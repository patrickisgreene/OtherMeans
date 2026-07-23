use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TimeConfig {
    pub day: usize,
    pub year: usize,
    pub month: usize,
    pub days_per_year: usize,
    pub hours_per_day: usize,
    pub days_per_month: usize,
    pub months_per_year: usize,
    pub seconds_per_hour: usize,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            day: Default::default(),
            year: Default::default(),
            month: Default::default(),
            days_per_year: 365,
            hours_per_day: 24,
            days_per_month: 30,
            months_per_year: 12,
            seconds_per_hour: 30,
        }
    }
}

impl TimeConfig {
    pub fn new(year: usize, month: usize, day: usize) -> TimeConfig {
        TimeConfig {
            year,
            month,
            day,
            ..default()
        }
    }
}
