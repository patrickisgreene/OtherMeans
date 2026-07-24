use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{AsBindGroup, BindGroupEntries},
        renderer::RenderDevice,
        storage::GpuShaderBuffer,
    },
};

use crate::render::{GpuTerrain, TerrainBindGroup};

pub fn prepare(
    device: Res<RenderDevice>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    mut gpu_terrains: Query<&mut GpuTerrain>,
) {
    for mut gpu_terrain in &mut gpu_terrains {
        let terrain_buffer = buffers.get(&gpu_terrain.terrain_buffer).unwrap();

        // Todo: be smarter about bind group recreation
        gpu_terrain.terrain_bind_group = Some(device.create_bind_group(
            "terrain_bind_group",
            &TerrainBindGroup::bind_group_layout(&device),
            &BindGroupEntries::sequential((
                terrain_buffer.buffer.as_entire_binding(),
                &gpu_terrain.attachment_buffer,
                &gpu_terrain.atlas_sampler,
                &gpu_terrain.attachment_textures[0],
                &gpu_terrain.attachment_textures[1],
                &gpu_terrain.attachment_textures[2],
                &gpu_terrain.attachment_textures[3],
                &gpu_terrain.attachment_textures[4],
                &gpu_terrain.attachment_textures[5],
                &gpu_terrain.attachment_textures[6],
                &gpu_terrain.attachment_textures[7],
            )),
        ));
    }
}
