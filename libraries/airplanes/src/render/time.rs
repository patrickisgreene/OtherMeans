use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{SystemParamItem, lifetimeless::SRes},
    },
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};
use terrain::prelude::TileTree;

use crate::render::pipeline::RenderParamsBindGroupLayout;

/// The single per-frame input driving all plane rendering: `shaders/airplanes.wgsl` computes each
/// plane's position as a function of `elapsed_secs`, its per-instance speed and phase, and its
/// static chain waypoints. `blend_distance`/`blend_range`/`max_lod` are terrain's own LOD
/// blend-region parameters, folded into the same uniform so planes fade out in sync with terrain
/// LOD. Copy of `shipping::render::time::ShippingRenderParams`.
#[derive(Resource, Clone, Copy, ExtractResource, ShaderType)]
pub struct AirplaneRenderParams {
    pub elapsed_secs: f32,
    pub blend_distance: f32,
    pub blend_range: f32,
    pub max_lod: f32,
}

/// `shaders/airplanes.wgsl`'s `compute_fade` divides by `blend_distance` and `blend_range` - a
/// derived, all-zero `Default` (as `shipping`'s equivalent uses) would make every plane fully
/// transparent (`log2(0/x)` = -inf, `saturate` clamps that to alpha 0) on any frame this resource
/// is read before `update_airplane_render_params` has found a `TileTree` to copy real values
/// from. Explicit large fallbacks here mean a plane renders fully opaque instead of invisible
/// during that window, regardless of whether it turns out to be the whole story.
impl Default for AirplaneRenderParams {
    fn default() -> Self {
        Self {
            elapsed_secs: 0.0,
            blend_distance: f32::MAX,
            blend_range: 1.0,
            max_lod: f32::MAX,
        }
    }
}

/// Mirrors `shipping::render::time::update_shipping_render_params` - there's only ever one
/// terrain entity in this app.
pub fn update_airplane_render_params(
    mut params: ResMut<AirplaneRenderParams>,
    time: Res<Time>,
    tile_trees: Query<&TileTree>,
) {
    params.elapsed_secs = time.elapsed_secs();

    if let Some(tile_tree) = tile_trees.iter().next() {
        params.blend_distance = tile_tree.blend_distance as f32;
        params.blend_range = tile_tree.blend_range;
        params.max_lod = tile_tree.lod_count.saturating_sub(1) as f32;
    }
}

#[derive(Resource, Default)]
pub struct RenderParamsBuffer(pub UniformBuffer<AirplaneRenderParams>);

pub fn prepare_render_params_buffer(
    params: Res<AirplaneRenderParams>,
    mut buffer: ResMut<RenderParamsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    buffer.0.set(*params);
    buffer.0.write_buffer(&render_device, &render_queue);
}

#[derive(Resource)]
pub struct RenderParamsBindGroup(pub BindGroup);

pub fn prepare_render_params_bind_group(
    mut commands: Commands,
    layout: Res<RenderParamsBindGroupLayout>,
    buffer: Res<RenderParamsBuffer>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    if let Some(binding) = buffer.0.binding() {
        commands.insert_resource(RenderParamsBindGroup(render_device.create_bind_group(
            "AirplaneRenderParams bindgroup",
            &pipeline_cache.get_bind_group_layout(&layout.layout),
            &BindGroupEntries::single(binding),
        )));
    }
}

pub struct SetRenderParamsBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetRenderParamsBindGroup<I> {
    type Param = SRes<RenderParamsBindGroup>;
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
