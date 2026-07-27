use bevy::{
    feathers::{rounded_corners::RoundedCorners, theme::ThemeBackgroundColor, tokens},
    prelude::*,
};

#[derive(Component, Debug)]
#[require(InheritedVisibility, Node::default())]
pub struct LabelsRoot;

#[derive(Component, Debug, Deref)]
#[relationship_target(relationship = LabelFor, linked_spawn)]
pub struct HasLabel(Entity);

#[derive(Component, Debug, Clone, Deref)]
#[require(Node {
    position_type: PositionType::Absolute,
    padding: UiRect::all(px(4)),
    border_radius: {RoundedCorners::All.to_border_radius(4.0)},
    ..Default::default()
})]
#[require(ThemeBackgroundColor(tokens::PANE_BODY_BG))]
#[relationship(relationship_target = HasLabel)]
pub struct LabelFor(pub Entity);

impl Default for LabelFor {
    // bsn! requires every component type to implement Default even when a scene always provides
    // an explicit value (see label_scene) - never actually read since label_scene always
    // overrides it with the real source entity.
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}
