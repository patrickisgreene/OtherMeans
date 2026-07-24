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
use terrain::prelude::{TerrainViewComponents, TileTree};

use crate::render::pipeline::RenderParamsBindGroupLayout;

/// The single per-frame input driving all vehicle rendering: `shaders/vehicles.wgsl` computes
/// each vehicle's position as a function of `elapsed_secs`, its per-instance speed and phase, and
/// its static chain waypoints - so moving every vehicle costs nothing more per frame than
/// uploading this one small uniform. `blend_distance`/`blend_range`/`max_lod` are terrain's own
/// LOD blend-region parameters (mirrors `buildings::render::fade::BuildingsFadeParams`), folded
/// into the same uniform rather than a second bind group, so vehicles fade out in sync with
/// terrain LOD exactly like buildings do, without a second per-frame upload.
#[derive(Resource, Clone, Copy, Default, ExtractResource, ShaderType)]
pub struct VehiclesRenderParams {
    pub elapsed_secs: f32,
    pub blend_distance: f32,
    pub blend_range: f32,
    pub max_lod: f32,
}

/// Mirrors `buildings::instances::update_building_batches`'s per-frame fade-param refresh -
/// there's only ever one (terrain, view) pair in this app.
pub fn update_vehicles_render_params(
    mut params: ResMut<VehiclesRenderParams>,
    time: Res<Time>,
    tile_trees: Res<TerrainViewComponents<TileTree>>,
) {
    params.elapsed_secs = time.elapsed_secs();

    if let Some(tile_tree) = tile_trees.values().next() {
        params.blend_distance = tile_tree.blend_distance as f32;
        params.blend_range = tile_tree.blend_range;
        params.max_lod = tile_tree.lod_count.saturating_sub(1) as f32;
    }
}

#[derive(Resource, Default)]
pub struct RenderParamsBuffer(pub UniformBuffer<VehiclesRenderParams>);

pub fn prepare_render_params_buffer(
    params: Res<VehiclesRenderParams>,
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
            "VehiclesRenderParams bindgroup",
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
