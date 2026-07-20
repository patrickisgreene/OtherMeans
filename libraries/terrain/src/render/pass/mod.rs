use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_phase::ViewSortedRenderPhases,
        render_resource::*,
        renderer::{RenderContext, ViewQuery},
        sync_world::MainEntity,
        view::{RetainedViewEntity, ViewDepthTexture, ViewTarget},
    },
};

mod depth;
mod item;
pub mod systems;

pub use depth::*;
pub use item::*;

pub(crate) const TERRAIN_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32FloatStencil8;

pub fn terrain_pass(
    world: &World,
    view: ViewQuery<(
        MainEntity,
        &ExtractedCamera,
        &ViewTarget,
        &ViewDepthTexture,
        &TerrainViewDepthTexture,
    )>,
    mut ctx: RenderContext,
) {
    let render_view = view.entity();
    let (main_view, camera, target, depth, terrain_depth) = view.into_inner();

    let pipeline_cache = world.resource::<PipelineCache>();
    let depth_copy_pipeline = world.resource::<DepthCopyPipeline>();

    let Some(pipeline) = pipeline_cache.get_render_pipeline(depth_copy_pipeline.id) else {
        return;
    };

    let Some(terrain_phase) = world
        .get_resource::<ViewSortedRenderPhases<TerrainItem>>()
        .and_then(|phase| {
            phase.get(&RetainedViewEntity {
                main_entity: main_view.into(),
                auxiliary_entity: Entity::PLACEHOLDER.into(),
                subview_index: 0,
            })
        })
    else {
        return;
    };

    if terrain_phase.items.is_empty() {
        return;
    }

    // Todo: prepare this in a separate system
    let terrain_depth_view = terrain_depth.texture.create_view(&TextureViewDescriptor {
        aspect: TextureAspect::DepthOnly,
        ..default()
    });
    let depth_copy_bind_group = ctx.render_device().create_bind_group(
        None,
        &depth_copy_pipeline.layout,
        &BindGroupEntries::single(&terrain_depth_view),
    );

    let color_attachments = [Some(target.get_color_attachment())];
    let terrain_depth_stencil_attachment = Some(terrain_depth.get_attachment());
    let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

    {
        let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("terrain_pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: terrain_depth_stencil_attachment,
            ..default()
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        terrain_phase
            .render(&mut render_pass, world, render_view)
            .unwrap();
    }

    {
        let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            depth_stencil_attachment,
            ..default()
        });
        render_pass.set_bind_group(0, &depth_copy_bind_group, &[]);
        render_pass.set_render_pipeline(pipeline);
        render_pass.draw(0..3, 0..1);
    }
}
