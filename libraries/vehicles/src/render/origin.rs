use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{extract_component::ExtractComponent, sync_component::SyncComponent},
};

use crate::instances::VehicleInstances;

/// The camera-relative (big_space floating-origin-corrected) world position of a vehicle batch's
/// tile center, re-extracted from the entity's [`GlobalTransform`] every frame so it stays
/// correct as the floating origin moves. Copy of `buildings::render::origin::BuildingTileOrigin`
/// - cheap (one `Vec3` per tile) so re-extracting it unconditionally every frame is fine, unlike
/// [`VehicleInstances`].
#[derive(Component, Clone, Copy)]
pub struct VehicleTileOrigin {
    pub translation: Vec3,
}

impl SyncComponent for VehicleTileOrigin {
    type Target = VehicleTileOrigin;
}

impl ExtractComponent for VehicleTileOrigin {
    type QueryData = &'static GlobalTransform;
    type QueryFilter = With<VehicleInstances>;
    type Out = VehicleTileOrigin;

    fn extract_component(transform: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(VehicleTileOrigin {
            translation: transform.translation(),
        })
    }
}
