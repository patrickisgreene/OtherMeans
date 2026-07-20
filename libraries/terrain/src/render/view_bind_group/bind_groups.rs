use bevy::{
    prelude::*,
    render::{
        render_resource::{AsBindGroup, Buffer},
        storage::ShaderBuffer,
    },
};

#[derive(AsBindGroup)]
pub struct IndirectBindGroup {
    #[storage(0, visibility(compute), buffer)]
    pub(crate) indirect: Buffer,
}

#[derive(AsBindGroup)]
pub struct PrepassViewBindGroup {
    #[storage(0, visibility(compute), read_only)]
    pub(crate) terrain_view: Handle<ShaderBuffer>,
    #[storage(1, visibility(compute))]
    pub(crate) approximate_height: Handle<ShaderBuffer>,
    #[storage(2, visibility(compute), read_only)]
    pub(crate) tile_tree: Handle<ShaderBuffer>,
    #[storage(3, visibility(compute), buffer)]
    pub(crate) final_tiles: Buffer,
    #[storage(4, visibility(compute), buffer)]
    pub(crate) temporary_tiles: Buffer,
    #[storage(5, visibility(compute), buffer)]
    pub(crate) state: Buffer,
}

#[derive(AsBindGroup)]
pub struct TerrainViewBindGroup {
    // Todo: replace with updatable uniform buffer
    #[storage(0, visibility(vertex, fragment), read_only)]
    pub(crate) terrain_view: Handle<ShaderBuffer>,
    #[storage(1, visibility(vertex, fragment), read_only)]
    pub(crate) approximate_height: Handle<ShaderBuffer>,
    #[storage(2, visibility(vertex, fragment), read_only)]
    pub(crate) tile_tree: Handle<ShaderBuffer>,
    #[storage(3, visibility(vertex, fragment), read_only, buffer)]
    pub(crate) geometry_tiles: Buffer,
}
