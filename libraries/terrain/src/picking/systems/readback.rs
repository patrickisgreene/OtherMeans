use crate::picking::{GpuPickingData, PickingData};
use bevy::{prelude::*, render::gpu_readback::ReadbackComplete};
use big_space::prelude::*;

pub fn picking_readback(trigger: On<ReadbackComplete>, mut picking_data: Query<&mut PickingData>) {
    let GpuPickingData {
        cursor_coords,
        depth,
        stencil: _stencil,
        world_from_clip,
        cell,
    } = trigger.event().to_shader_type();

    let ndc_coords = (2.0 * cursor_coords - 1.0).extend(depth);

    let mut picking_data = picking_data.get_mut(trigger.entity).unwrap();
    picking_data.cursor_coords = cursor_coords;
    picking_data.cell = CellCoord::new(cell.x, cell.y, cell.z);
    picking_data.translation = (depth > 0.0).then(|| world_from_clip.project_point3(ndc_coords));
    picking_data.world_from_clip = world_from_clip;
}
