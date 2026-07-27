use bevy::feathers::display::*;
use bevy::prelude::*;
use cities::{CityLabelRank, CityScaleRank};
use terrain::TerrainConfigHandle;
use terrain::view::TerrainViewConfig;

use crate::{HasLabel, LabelFor, LabelsRoot, visibility::visible};

pub fn update_labels(
    mut commands: Commands,
    root: Query<Entity, With<LabelsRoot>>,
    camera: Query<(&Camera, &GlobalTransform, Option<&TerrainViewConfig>)>,
    occluder: Query<&GlobalTransform, With<TerrainConfigHandle>>,
    unlabelled: Query<
        (
            Entity,
            &Name,
            &GlobalTransform,
            &CityLabelRank,
            Option<&CityScaleRank>,
        ),
        Without<HasLabel>,
    >,
    labelled: Query<(&GlobalTransform, &HasLabel, Option<&CityScaleRank>), With<CityLabelRank>>,
    orphaned: Query<&HasLabel, Without<CityLabelRank>>,
) {
    for has_label in &orphaned {
        commands.entity(**has_label).despawn();
    }

    let (Ok((camera, camera_transform, view_config)), Ok(root)) = (camera.single(), root.single())
    else {
        return;
    };
    let occluder_pos = occluder.single().ok().map(GlobalTransform::translation);

    for (transform, has_label, lod) in &labelled {
        if !visible(
            camera,
            camera_transform,
            view_config,
            occluder_pos,
            transform,
            lod,
        ) {
            commands.entity(**has_label).despawn();
        }
    }

    for (entity, name, transform, priority, lod) in &unlabelled {
        if !visible(
            camera,
            camera_transform,
            view_config,
            occluder_pos,
            transform,
            lod,
        ) {
            continue;
        }
        let z_index = i32::from(u8::MAX - priority.0);

        commands.spawn_scene(bsn! {
            ChildOf(root)
            LabelFor(entity)
            ZIndex(z_index)
            Children [
                label_small(name)
            ]
        });
    }
}
