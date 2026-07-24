use std::marker::PhantomData;

use crate::systems;
use bevy::prelude::*;

#[derive(Component, PartialEq)]
pub struct NoTag;

pub struct MetricsPlugin<const S: &'static str, T: Component + PartialEq = NoTag, L = Update> {
    label: L,
    tag: PhantomData<T>,
}

impl<const S: &'static str, T: Component + PartialEq, L> MetricsPlugin<S, T, L> {
    pub fn new(label: L) -> MetricsPlugin<S, T, L> {
        Self {
            label,
            tag: Default::default(),
        }
    }
}

impl<const S: &'static str, T: Component + PartialEq> Default for MetricsPlugin<S, T> {
    fn default() -> Self {
        Self {
            label: Update,
            tag: Default::default(),
        }
    }
}

impl<const S: &'static str, T: Component + PartialEq> Plugin for MetricsPlugin<S, T> {
    fn build(&self, app: &mut App) {
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<NoTag>() {
            app.add_systems(
                self.label.clone(),
                (
                    systems::init_cumulative_components_simple::<S>,
                    systems::init_components_simple::<S>,
                    systems::update_components_simple::<S>,
                    systems::remove_component_simple::<S>,
                )
                    .chain(),
            );
        } else {
            app.add_systems(
                self.label.clone(),
                (
                    systems::init_cumulative_components_tagged::<S, T>,
                    systems::init_components_tagged::<S, T>,
                    systems::update_components_tagged::<S, T>,
                    systems::remove_component_tagged::<S, T>,
                )
                    .chain(),
            );
        }
    }
}
