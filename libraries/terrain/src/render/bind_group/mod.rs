use crate::data::TileAtlas;
use bevy::{
    ecs::query::ROQueryItem,
    math::{Affine3, Affine3Ext},
    prelude::*,
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
    },
};

mod attachment;
mod gpu;
pub mod systems;

pub use attachment::*;
pub use gpu::*;

// Todo: use this once texture views can be used directly
#[derive(AsBindGroup)]
pub struct TerrainBindGroup {
    #[storage(0, visibility(all), read_only, buffer)]
    pub terrain: Buffer,
    #[uniform(1, visibility(all))]
    pub attachments: AttachmentUniform,
    #[sampler(2, visibility(all))]
    #[texture(3, visibility(all), dimension = "2d_array")]
    pub attachment0: Handle<Image>,
    #[texture(4, visibility(all), dimension = "2d_array")]
    pub attachment1: Handle<Image>,
    #[texture(5, visibility(all), dimension = "2d_array")]
    pub attachment2: Handle<Image>,
    #[texture(6, visibility(all), dimension = "2d_array")]
    pub attachment3: Handle<Image>,
    #[texture(7, visibility(all), dimension = "2d_array")]
    pub attachment4: Handle<Image>,
    #[texture(8, visibility(all), dimension = "2d_array")]
    pub attachment5: Handle<Image>,
    #[texture(9, visibility(all), dimension = "2d_array")]
    pub attachment6: Handle<Image>,
    #[texture(10, visibility(all), dimension = "2d_array")]
    pub attachment7: Handle<Image>,
}

/// The terrain config data that is available in shaders.
#[derive(Default, ShaderType)]
pub struct TerrainUniform {
    lod_count: u32,
    scale: Vec3,
    min_height: f32,
    max_height: f32,
    height_scale: f32,
    world_from_local: [Vec4; 3],
    local_from_world_transpose_a: [Vec4; 2],
    local_from_world_transpose_b: f32,
}

impl TerrainUniform {
    pub fn new(tile_atlas: &TileAtlas, global_transform: &GlobalTransform) -> Self {
        let transform = Affine3::from(global_transform.affine());
        let world_from_local = transform.to_transpose();
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            transform.inverse_transpose_3x3();

        Self {
            lod_count: tile_atlas.lod_count,
            scale: tile_atlas.shape.scale().as_vec3(),
            min_height: tile_atlas.min_height * tile_atlas.height_scale,
            max_height: tile_atlas.max_height * tile_atlas.height_scale,
            height_scale: tile_atlas.height_scale,
            world_from_local,
            local_from_world_transpose_a,
            local_from_world_transpose_b,
        }
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = &'static GpuTerrain;

    #[inline]
    fn render<'w, 's>(
        _item: &P,
        _: ROQueryItem<'w, 's, Self::ViewQuery>,
        gpu_terrain: Option<ROQueryItem<'w, 's, Self::ItemQuery>>,
        _: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(gpu_terrain) = gpu_terrain else {
            return RenderCommandResult::Skip;
        };

        if let Some(bind_group) = &gpu_terrain.terrain_bind_group {
            pass.set_bind_group(I, bind_group, &[]);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Skip
        }
    }
}
