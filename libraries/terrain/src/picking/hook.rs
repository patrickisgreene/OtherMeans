use bevy::{
    asset::RenderAssetUsages,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
    render::{
        gpu_readback::Readback,
        render_resource::{BufferUsages, ShaderType},
        storage::ShaderBuffer,
    },
};

use crate::picking::data::{GpuPickingData, PickingData};

use super::systems;

pub fn picking_hook(mut world: DeferredWorld, context: HookContext) {
    let mut buffers = world.resource_mut::<Assets<ShaderBuffer>>();
    let mut buffer = ShaderBuffer::with_size(
        GpuPickingData::min_size().get() as usize,
        RenderAssetUsages::default(),
    );
    buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);

    world
        .commands()
        .entity(context.entity)
        .insert(Readback::buffer(buffer.clone()))
        .observe(systems::picking_readback);

    let mut picking_data = world.get_mut::<PickingData>(context.entity).unwrap();
    picking_data.buffer = buffer;
}
