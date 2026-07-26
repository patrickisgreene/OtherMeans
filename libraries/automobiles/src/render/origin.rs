use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{extract_component::ExtractComponent, sync_component::SyncComponent},
};

use crate::instances::AutomobilesInstances;

/// The camera-relative (big_space floating-origin-corrected) world position of a automobile batch's
/// tile center, re-extracted from the entity's [`GlobalTransform`] every frame so it stays
/// correct as the floating origin moves. Copy of `buildings::render::origin::BuildingTileOrigin`
/// - cheap (one `Vec3` per tile) so re-extracting it unconditionally every frame is fine, unlike
/// [`AutomobilesInstances`].
#[derive(Component, Clone, Copy)]
pub struct AutomobilesTileOrigin {
    pub translation: Vec3,
}

impl SyncComponent for AutomobilesTileOrigin {
    type Target = AutomobilesTileOrigin;
}

impl ExtractComponent for AutomobilesTileOrigin {
    type QueryData = &'static GlobalTransform;
    type QueryFilter = With<AutomobilesInstances>;
    type Out = AutomobilesTileOrigin;

    fn extract_component(transform: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(AutomobilesTileOrigin {
            translation: transform.translation(),
        })
    }
}
