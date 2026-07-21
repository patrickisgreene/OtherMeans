use bevy::{
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_asset::RenderAssets,
        render_resource::{
            BindGroupEntries, Operations, PipelineCache, RenderPassColorAttachment,
            RenderPassDescriptor,
        },
        renderer::{RenderContext, ViewQuery},
        texture::{FallbackImage, GpuImage},
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};
use terrain::render::pass::TerrainViewDepthTexture;

use super::{
    EarthAtmosphereSettings,
    pipeline::{EarthAtmospherePipeline, WindTexture},
};

/// Applies the atmosphere post-process effect for the current view.
///
/// Registered directly into the `Core3d` schedule (see plugin.rs) rather than the old
/// render-graph node system, which bevy removed in favor of plain systems ordered within
/// per-camera schedules. `ViewQuery` reads the `CurrentView` resource set by
/// `bevy_core_pipeline::schedule::camera_driver` and only runs this system for views matching
/// the query - `DynamicUniformIndex<PostProcessSettings>` only exists on views that were
/// extracted with a `PostProcessSettings` component, so this is automatically skipped for
/// cameras without the effect enabled (mirrors `TerrainMainNode`'s old `ViewQuery` marker).
pub fn post_process_pass(
    world: &World,
    view: ViewQuery<(
        &ViewTarget,
        &EarthAtmosphereSettings,
        &DynamicUniformIndex<EarthAtmosphereSettings>,
        &ViewUniformOffset,
        &TerrainViewDepthTexture,
    )>,
    mut render_context: RenderContext,
) {
    let (view_target, _post_process_settings, settings_index, view_uniform_offset, earth_depth) =
        view.into_inner();

    // Get the pipeline resource that contains the global data we need
    // to create the render pipeline
    let post_process_pipeline = world.resource::<EarthAtmospherePipeline>();

    // The pipeline cache is a cache of all previously created pipelines.
    // It is required to avoid creating a new pipeline each frame,
    // which is expensive due to shader compilation.
    let pipeline_cache = world.resource::<PipelineCache>();

    // Get the pipeline from the cache
    let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id)
    else {
        return;
    };

    // Resolve the concrete bind group layout from its descriptor (cached by the pipeline
    // cache, cheap to call repeatedly - see PostProcessPipeline's doc comment).
    let layout = pipeline_cache.get_bind_group_layout(&post_process_pipeline.layout);

    // Get the settings uniform binding
    let settings_uniforms = world.resource::<ComponentUniforms<EarthAtmosphereSettings>>();
    let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
        return;
    };

    // Get the view uniforms binding
    let view_uniforms = world.resource::<ViewUniforms>();
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    // Wind-flow texture used to advect the cloud noise - falls back to the engine's default
    // fallback image for the one or two frames before it finishes loading, same pattern
    // terrain's own GpuTerrain::new uses for its attachment textures (see
    // libraries/terrain/src/render/bind_group/gpu.rs).
    let wind_texture = world.resource::<WindTexture>();
    let gpu_images = world.resource::<RenderAssets<GpuImage>>();
    let fallback_image = world.resource::<FallbackImage>();
    let wind_gpu_image = gpu_images
        .get(&wind_texture.0)
        .unwrap_or(&fallback_image.d2);

    // This will start a new "post process write", obtaining two texture
    // views from the view target - a `source` and a `destination`.
    // `source` is the "current" main texture and you _must_ write into
    // `destination` because calling `post_process_write()` on the
    // [`ViewTarget`] will internally flip the [`ViewTarget`]'s main
    // texture to the `destination` texture. Failing to do so will cause
    // the current main texture information to be lost.
    let post_process = view_target.post_process_write();

    // The bind_group gets created each frame.
    //
    // Normally, you would create a bind_group in the Queue set,
    // but this doesn't work with the post_process_write().
    // The reason it doesn't work is because each post_process_write will alternate the source/destination.
    // The only way to have the correct source/destination for the bind_group
    // is to make sure you get it during the pass's own execution.
    let bind_group = render_context.render_device().create_bind_group(
        "post_process_bind_group",
        &layout,
        // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
        &BindGroupEntries::sequential((
            // binding 0: Make sure to use the source view
            post_process.source,
            // binding 1: Use the sampler created for the pipeline
            &post_process_pipeline.sampler,
            // binding 2: Set the earth depth texture
            &earth_depth.depth_view,
            // binding 3: Set the view uniform binding
            view_binding.clone(),
            // binding 4: Set the settings binding
            settings_binding.clone(),
            // binding 5: The wind-flow texture view
            &wind_gpu_image.texture_view,
            // binding 6: The wind-flow texture's sampler
            &wind_gpu_image.sampler,
        )),
    );

    // Begin the render pass
    let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("post_process_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            // We need to specify the post process destination view here
            // to make sure we write to the appropriate texture.
            view: post_process.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    // This is mostly just wgpu boilerplate for drawing a fullscreen triangle,
    // using the pipeline/bind_group created above
    render_pass.set_render_pipeline(pipeline);
    // Dynamic offsets must be provided in binding order:
    // - binding 3: view uniform offset
    // - binding 4: settings uniform offset
    render_pass.set_bind_group(
        0,
        &bind_group,
        &[view_uniform_offset.offset, settings_index.index()],
    );
    render_pass.draw(0..3, 0..1);
}
