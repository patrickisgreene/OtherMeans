use bevy::prelude::*;

mod plugin;
mod systems;

pub use plugin::LabelsPlugin;

#[derive(Component, Debug)]
#[require(InheritedVisibility, Node::default())]
pub struct LabelsRoot;

#[derive(Component, Debug)]
#[relationship_target(relationship = LabelFor, linked_spawn)]
pub struct HasLabel(Entity);

#[derive(Component, Debug, Clone)]
#[relationship(relationship_target = HasLabel)]
pub struct LabelFor(Entity);

impl Default for LabelFor {
    // bsn! requires every component type to implement Default even when a scene always provides
    // an explicit value (see label_scene) - never actually read since label_scene always
    // overrides it with the real source entity.
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}
