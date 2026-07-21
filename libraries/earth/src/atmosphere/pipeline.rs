use bevy::{
    core_pipeline::FullscreenShader,
    material::descriptor::BindGroupLayoutDescriptor,
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{
            BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FragmentState, PipelineCache, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, TextureFormat, TextureSampleType,
            binding_types::{sampler, texture_2d, texture_depth_2d_multisampled, uniform_buffer},
        },
        renderer::RenderDevice,
        view::ViewUniform,
    },
};

use super::{EarthAtmosphereSettings, SHADER_ASSET_PATH};

/// Handle to the small equirectangular wind-flow texture (see
/// assets/textures/earth/wind-flow.png) sampled by the atmosphere shader to advect the cloud
/// noise - a static, one-off asset (not part of the regular terrain baking pipeline), loaded
/// once in `EarthAtmospherePlugin::build` and extracted into the render world every frame like
/// `EarthAtmosphereSettings`.
#[derive(Resource, Clone, ExtractResource)]
pub struct WindTexture(pub Handle<Image>);

// This contains global data used by the render pipeline. This will be created once on startup.
//
// `layout` is a descriptor rather than a concrete `BindGroupLayout` - render/compute pipeline
// descriptors in this bevy version take `Vec<BindGroupLayoutDescriptor>` (see
// bevy_material::descriptor::RenderPipelineDescriptor), and the concrete layout is resolved
// on demand via `PipelineCache::get_bind_group_layout` wherever an actual bind group needs to
// be created (see pass.rs) - mirrors how terrain's own mip pipelines do this (see
// libraries/terrain/src/mipmap/{mod.rs,pipelines.rs}).
#[derive(Resource)]
pub struct EarthAtmospherePipeline {
    pub layout: BindGroupLayoutDescriptor,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}
impl EarthAtmospherePipeline {
    pub fn initialize(
        mut commands: Commands,
        render_device: Res<RenderDevice>,
        asset_server: Res<AssetServer>,
        fullscreen_shader: Res<FullscreenShader>,
        pipeline_cache: Res<PipelineCache>,
    ) {
        // We need to define the bind group layout used for our pipeline
        let layout = BindGroupLayoutDescriptor::new(
            "post_process_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // binding 0: The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // binding 1: The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::Filtering),
                    // binding 2: The earth depth texture (multisampled)
                    texture_depth_2d_multisampled(),
                    // binding 3: The view uniform (camera matrices, etc.)
                    uniform_buffer::<ViewUniform>(true),
                    // binding 4: The settings uniform that will control the effect
                    uniform_buffer::<EarthAtmosphereSettings>(true),
                    // binding 5: The wind-flow texture used to advect the cloud noise
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // binding 6: The wind-flow texture's own sampler (repeats over longitude)
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        // Get the shader handle
        let shader = asset_server.load(SHADER_ASSET_PATH);
        // This will setup a fullscreen triangle for the vertex state.
        let vertex_state = fullscreen_shader.to_vertex_state();
        let pipeline_id = pipeline_cache
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("post_process_pipeline".into()),
                layout: vec![layout.clone()],
                vertex: vertex_state,
                fragment: Some(FragmentState {
                    shader,
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    targets: vec![Some(ColorTargetState {
                        // Bevy no longer encourages a single default swapchain format (see
                        // ExtractedView::texture_format for the real per-view format); this
                        // matches what the deprecated bevy_default() used to always return.
                        format: TextureFormat::Rgba8UnormSrgb,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    ..default()
                }),
                ..default()
            });
        commands.insert_resource(EarthAtmospherePipeline {
            layout,
            sampler,
            pipeline_id,
        });
    }
}
