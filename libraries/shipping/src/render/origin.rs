use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{extract_component::ExtractComponent, sync_component::SyncComponent},
};

use crate::instances::ShippingInstances;

/// The camera-relative (big_space floating-origin-corrected) world position of a ship batch's
/// tile center, re-extracted from the entity's [`GlobalTransform`] every frame so it stays
/// correct as the floating origin moves. Copy of `buildings::render::origin::BuildingTileOrigin`
/// - cheap (one `Vec3` per tile) so re-extracting it unconditionally every frame is fine, unlike
/// [`ShippingInstances`].
#[derive(Component, Clone, Copy)]
pub struct ShippingTileOrigin {
    pub translation: Vec3,
}

impl SyncComponent for ShippingTileOrigin {
    type Target = ShippingTileOrigin;
}

impl ExtractComponent for ShippingTileOrigin {
    type QueryData = &'static GlobalTransform;
    type QueryFilter = With<ShippingInstances>;
    type Out = ShippingTileOrigin;

    fn extract_component(transform: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(ShippingTileOrigin {
            translation: transform.translation(),
        })
    }
}
