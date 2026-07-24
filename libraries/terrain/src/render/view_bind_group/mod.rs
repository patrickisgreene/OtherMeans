use bevy::{
    ecs::query::ROQueryItem,
    prelude::*,
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
    },
};

mod bind_groups;
mod gpu;
pub mod systems;
mod uniform;

pub use bind_groups::*;
pub use gpu::*;
pub use uniform::*;

#[derive(ShaderType)]
pub(crate) struct GeometryTile {
    face: u32,
    lod: u32,
    xy: UVec2,
    view_distances: Vec4,
    morph_ratios: Vec4,
}

#[derive(ShaderType)]
pub(crate) struct Indirect {
    x_or_vertex_count: u32,
    y_or_instance_count: u32,
    z_or_base_vertex: u32,
    base_instance: u32,
}

#[derive(ShaderType)]
pub(crate) struct PrepassState {
    tile_count: u32,
    counter: i32,
    child_index: i32,
    final_index: i32,
}

pub struct SetTerrainViewBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainViewBindGroup<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = &'static GpuTerrainView;

    #[inline]
    fn render<'w, 's>(
        _item: &P,
        _view: ROQueryItem<'w, 's, Self::ViewQuery>,
        gpu_terrain_view: Option<ROQueryItem<'w, 's, Self::ItemQuery>>,
        _: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(gpu_terrain_view) = gpu_terrain_view else {
            return RenderCommandResult::Skip;
        };

        if let Some(bind_group) = &gpu_terrain_view.terrain_view_bind_group {
            pass.set_bind_group(I, bind_group, &[]);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Skip
        }
    }
}

pub(crate) struct DrawTerrainCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawTerrainCommand {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = &'static GpuTerrainView;

    #[inline]
    fn render<'w, 's>(
        _item: &P,
        _view: ROQueryItem<'w, 's, Self::ViewQuery>,
        gpu_terrain_view: Option<ROQueryItem<'w, 's, Self::ItemQuery>>,
        _: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(gpu_terrain_view) = gpu_terrain_view else {
            return RenderCommandResult::Skip;
        };

        pass.set_stencil_reference(gpu_terrain_view.order);
        pass.draw_indirect(&gpu_terrain_view.indirect_buffer, 0);

        RenderCommandResult::Success
    }
}
