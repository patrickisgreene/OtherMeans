use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{extract_component::ExtractComponent, sync_component::SyncComponent},
};

use crate::instances::AirplaneInstances;

/// The camera-relative (big_space floating-origin-corrected) world position of a plane batch's
/// tile center, re-extracted from the entity's [`GlobalTransform`] every frame so it stays
/// correct as the floating origin moves. Copy of `shipping::render::origin::ShippingTileOrigin`.
#[derive(Component, Clone, Copy)]
pub struct AirplaneTileOrigin {
    pub translation: Vec3,
}

impl SyncComponent for AirplaneTileOrigin {
    type Target = AirplaneTileOrigin;
}

impl ExtractComponent for AirplaneTileOrigin {
    type QueryData = &'static GlobalTransform;
    type QueryFilter = With<AirplaneInstances>;
    type Out = AirplaneTileOrigin;

    fn extract_component(transform: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(AirplaneTileOrigin {
            translation: transform.translation(),
        })
    }
}
