use bevy::feathers::{
    display::*, rounded_corners::RoundedCorners, theme::ThemeBackgroundColor, tokens,
};
use bevy::prelude::*;
use cities::{CityLabelRank, CityScaleRank};
use terrain::TerrainConfigHandle;
use terrain::{math::TerrainShape, view::TerrainViewConfig};

use crate::{HasLabel, LabelFor, LabelsRoot};

fn label_scene(source: Entity, text: String, z_index: i32) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            padding: px(4),
            border_radius: {RoundedCorners::All.to_border_radius(4.0)}
        }
        ThemeBackgroundColor(tokens::PANE_BODY_BG)
        LabelFor(source)
        ZIndex(z_index)
        Children [
            label_small(text)
        ]
    }
}

fn on_screen(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    transform: &GlobalTransform,
) -> bool {
    let Ok(viewport_position) = camera.world_to_viewport(camera_transform, transform.translation())
    else {
        return false;
    };
    let Some(size) = camera.logical_viewport_size() else {
        return false;
    };
    viewport_position.cmpge(Vec2::ZERO).all() && viewport_position.cmple(size).all()
}

fn visible(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    view_config: Option<&TerrainViewConfig>,
    occluder_pos: Option<Vec3>,
    transform: &GlobalTransform,
    lod: Option<&CityScaleRank>,
) -> bool {
    if !on_screen(camera, camera_transform, transform) {
        return false;
    }

    let camera_pos = camera_transform.translation();
    let label_pos = transform.translation();

    if let Some(occluder_pos) = occluder_pos {
        let normal = (label_pos - occluder_pos).normalize_or_zero();
        if normal.dot(camera_pos - label_pos) <= 0.0 {
            return false;
        }
    }

    if let (Some(view_config), Some(lod)) = (view_config, lod) {
        let lod_distance = (view_config.blend_distance * TerrainShape::WGS84.face_size()) as f32;
        let level = (lod_distance / camera_pos.distance(label_pos))
            .log2()
            .floor()
            .max(0.0);
        if f32::from(lod.0) > level {
            return false;
        }
    }

    true
}

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
        commands.entity(has_label.0).despawn();
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
            commands.entity(has_label.0).despawn();
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
        commands
            .spawn_scene(label_scene(entity, name.to_string(), z_index))
            .insert(ChildOf(root));
    }
}
