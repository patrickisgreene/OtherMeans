use bevy::{
    prelude::*,
    render::{Extract, render_phase::ViewSortedRenderPhases, view::RetainedViewEntity},
};

use crate::render::TerrainItem;

pub fn extract_terrain_phases(
    mut terrain_phases: ResMut<ViewSortedRenderPhases<TerrainItem>>,
    cameras: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    terrain_phases.clear();

    for (entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }

        terrain_phases.insert(
            RetainedViewEntity {
                main_entity: entity.into(),
                auxiliary_entity: Entity::PLACEHOLDER.into(),
                subview_index: 0,
            },
            default(),
        );
    }
}
