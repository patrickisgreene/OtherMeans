use bevy::{prelude::*, render::storage::ShaderBuffer, window::PrimaryWindow};
use big_space::prelude::*;

use crate::picking::{GpuPickingData, PickingData};

pub fn picking_system(
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform, &CellCoord, &PickingData)>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Some(position) = window.cursor_position() else {
        return;
    };
    let cursor_coords = Vec2::new(position.x, window.size().y - position.y) / window.size();

    for (camera, global_transform, &cell, picking_data) in &camera {
        let mut buffer = buffers.get_mut(&picking_data.buffer).unwrap();
        let data = GpuPickingData {
            cursor_coords,
            depth: 0.0,
            stencil: 255,
            world_from_clip: global_transform.to_matrix() * camera.clip_from_view().inverse(),
            cell: IVec3::new(cell.x, cell.y, cell.z),
        };
        buffer.set_data(data);
    }
}
