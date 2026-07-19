use crate::{
    render::{TerrainViewDepthTexture, terrain_pass},
    shaders::PICKING_SHADER,
};
use bevy::{
    core_pipeline::{Core3dSystems, core_3d::main_opaque_pass_3d, schedule::Core3d},
    prelude::*,
    render::{
        RenderApp,
        extract_component::ExtractComponentPlugin,
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{
                storage_buffer, texture_2d_multisampled, texture_depth_2d_multisampled,
            },
            *,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        storage::GpuShaderBuffer,
    },
};

mod data;
mod hook;
mod systems;

pub use data::*;

#[derive(Resource)]
pub struct PickingPipeline {
    id: CachedComputePipelineId,
    layout: BindGroupLayout,
}

impl FromWorld for PickingPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let entries = BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_buffer::<data::GpuPickingData>(false),
                texture_depth_2d_multisampled(),
                texture_2d_multisampled(TextureSampleType::Uint),
            ),
        );

        let layout = device.create_bind_group_layout(None, &entries);

        let id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![BindGroupLayoutDescriptor::new(
                "picking_bind_group_layout",
                &entries,
            )],
            immediate_size: Default::default(),
            shader: world.load_asset(PICKING_SHADER),
            shader_defs: vec![],
            entry_point: Some("pick".into()),
            zero_initialize_workgroup_memory: false,
        });

        Self { id, layout }
    }
}

pub fn picking_pass(
    world: &World,
    view: ViewQuery<(&data::GpuPickingBuffer, &TerrainViewDepthTexture)>,
    mut ctx: RenderContext,
) {
    let pipeline_cache = world.resource::<PipelineCache>();
    let picking_pipeline = world.resource::<PickingPipeline>();
    let buffer = world.resource::<RenderAssets<GpuShaderBuffer>>();

    let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
        return;
    };

    let (picking_buffer, depth) = view.into_inner();

    let Some(buffer) = buffer.get(picking_buffer.0) else {
        return;
    };

    let bind_group = ctx.render_device().create_bind_group(
        None,
        &picking_pipeline.layout,
        &BindGroupEntries::sequential((
            buffer.buffer.as_entire_binding(),
            &depth.depth_view,
            &depth.stencil_view,
        )),
    );

    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());
    pass.set_bind_group(0, &bind_group, &[]);
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(1, 1, 1);
}

pub struct TerrainPickingPlugin;

impl Plugin for TerrainPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            systems::picking_system.after(TransformSystems::Propagate),
        )
        .add_plugins(ExtractComponentPlugin::<data::PickingData>::default());

        app.sub_app_mut(RenderApp).add_systems(
            Core3d,
            picking_pass
                .after(terrain_pass)
                .before(main_opaque_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
