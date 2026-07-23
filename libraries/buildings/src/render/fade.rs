use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};

use crate::render::pipeline::FadeParamsBindGroupLayout;

/// Terrain's own LOD blend-region parameters, re-extracted from the live `TileTree` every frame
/// (cheap - a handful of floats, unlike `BuildingInstances`) so building visibility fades out in
/// the same region terrain itself uses to blend LODs, instead of popping when a tile leaves the
/// active set. The actual fade math happens entirely in `shaders/buildings.wgsl`; this is just
/// the small, per-frame (not per-instance) input to it.
#[derive(Resource, Clone, Copy, ExtractResource, ShaderType)]
pub struct BuildingsFadeParams {
    pub blend_distance: f32,
    pub blend_range: f32,
    /// `lod_count - 1` - the LOD buildings are always placed on.
    pub max_lod: f32,
}

impl Default for BuildingsFadeParams {
    fn default() -> Self {
        Self {
            blend_distance: 1.0,
            blend_range: 1.0,
            max_lod: 0.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct FadeParamsBuffer(pub UniformBuffer<BuildingsFadeParams>);

pub fn prepare_fade_params_buffer(
    fade_params: Res<BuildingsFadeParams>,
    mut buffer: ResMut<FadeParamsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    buffer.0.set(*fade_params);
    buffer.0.write_buffer(&render_device, &render_queue);
}

#[derive(Resource)]
pub struct FadeParamsBindGroup(pub BindGroup);

pub fn prepare_fade_params_bind_group(
    mut commands: Commands,
    layout: Res<FadeParamsBindGroupLayout>,
    buffer: Res<FadeParamsBuffer>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    if let Some(binding) = buffer.0.binding() {
        commands.insert_resource(FadeParamsBindGroup(render_device.create_bind_group(
            "BuildingsFadeParams bindgroup",
            &pipeline_cache.get_bind_group_layout(&layout.layout),
            &BindGroupEntries::single(binding),
        )));
    }
}

pub struct SetFadeParamsBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetFadeParamsBindGroup<I> {
    type Param = SRes<FadeParamsBindGroup>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.into_inner().0, &[]);
        RenderCommandResult::Success
    }
}
