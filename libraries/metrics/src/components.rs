use bevy::prelude::*;

#[derive(Component, Default, Reflect, Deref, Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub struct Metric<const S: &'static str> {
    pub(crate) last: u8,
    #[deref]
    pub(crate) value: u8,
}

impl<const S: &'static str> Metric<S> {
    pub const MAX: u8 = u8::MAX;
    pub const MIN: u8 = u8::MIN;

    pub fn new(value: u8) -> Self {
        Self { last: value, value }
    }

    pub fn set(&mut self, value: u8) {
        self.last = self.value;
        self.value = value;
    }

    pub fn get(&self) -> u8 {
        self.value
    }
}

#[derive(Component, Default, Deref, Reflect, Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub struct CumulativeMetric<const S: &'static str> {
    pub(crate) max: u32,
    #[deref]
    pub(crate) value: u32,
}

impl<const S: &'static str> CumulativeMetric<S> {
    pub fn new(max: u32, value: u32) -> Self {
        Self { max, value }
    }
    pub fn get(&self) -> u32 {
        self.value
    }
    pub fn max(&self) -> u32 {
        self.max
    }
    pub fn normalized(&self) -> f32 {
        self.value as f32 / self.max as f32 * 100.0
    }
}
