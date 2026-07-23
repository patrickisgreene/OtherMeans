#![feature(adt_const_params, unsized_const_params)]
#![allow(incomplete_features)]

mod components;
mod plugin;
mod systems;

#[cfg(test)]
mod tests;

pub use components::*;
pub use plugin::StatisticsPlugin;
