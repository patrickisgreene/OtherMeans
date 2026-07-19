use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::ExtractComponent, render_resource::ShaderType, storage::ShaderBuffer,
        sync_component::SyncComponent,
    },
};
use big_space::prelude::*;

use super::hook::picking_hook;

#[derive(Default, Clone, Component)]
#[component(on_add = picking_hook)]
pub struct PickingData {
    pub cursor_coords: Vec2,
    pub cell: CellCoord,           // cell of floating origin (camera)
    pub translation: Option<Vec3>, // relative to floating origin cell
    pub world_from_clip: Mat4,
    pub(crate) buffer: Handle<ShaderBuffer>,
}

impl SyncComponent for PickingData {
    type Target = GpuPickingBuffer;
}

impl ExtractComponent for PickingData {
    type QueryData = &'static PickingData;
    type QueryFilter = ();
    type Out = GpuPickingBuffer;

    fn extract_component(data: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(GpuPickingBuffer(data.buffer.id()))
    }
}

#[derive(Component)]
pub struct GpuPickingBuffer(pub(crate) AssetId<ShaderBuffer>);

#[derive(Default, Debug, Clone, ShaderType)]
pub struct GpuPickingData {
    pub cursor_coords: Vec2,
    pub depth: f32,
    pub stencil: u32,
    pub world_from_clip: Mat4,
    pub cell: IVec3,
}
