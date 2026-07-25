use bevy::prelude::*;

use crate::LabelFor;

pub fn update_positions(
    camera: Query<(&Camera, &GlobalTransform)>,
    sources: Query<&GlobalTransform>,
    mut labels: Query<(&LabelFor, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };

    for (label_for, mut node) in &mut labels {
        let Ok(source_transform) = sources.get(label_for.0) else {
            continue;
        };

        if let Ok(viewport_position) =
            camera.world_to_viewport(camera_transform, source_transform.translation())
        {
            node.left = Val::Px(viewport_position.x);
            node.top = Val::Px(viewport_position.y);
        }
    }
}
