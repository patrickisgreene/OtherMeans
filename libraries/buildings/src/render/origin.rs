use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{extract_component::ExtractComponent, sync_component::SyncComponent},
};

use crate::instances::BuildingInstances;

/// The camera-relative (big_space floating-origin-corrected) world position of a building
/// batch's tile center, re-extracted from the entity's [`GlobalTransform`] every frame so it
/// stays correct as the floating origin moves. Cheap (one `Vec3` per tile) so re-extracting it
/// unconditionally every frame is fine, unlike [`BuildingInstances`]. Read by
/// `render::draw::prepare_merged_building_buffer` to bake each tile's instances into a single
/// shared absolute-position buffer on the CPU, rather than being bound as a GPU uniform.
#[derive(Component, Clone, Copy)]
pub struct BuildingTileOrigin {
    pub translation: Vec3,
}

impl SyncComponent for BuildingTileOrigin {
    type Target = BuildingTileOrigin;
}

impl ExtractComponent for BuildingTileOrigin {
    type QueryData = &'static GlobalTransform;
    type QueryFilter = With<BuildingInstances>;
    type Out = BuildingTileOrigin;

    fn extract_component(transform: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(BuildingTileOrigin {
            translation: transform.translation(),
        })
    }
}
