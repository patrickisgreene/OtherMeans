use bevy::{
    core_pipeline::{FullscreenShader, core_3d::CORE_3D_DEPTH_FORMAT},
    material::descriptor::{
        BindGroupLayoutDescriptor, CachedRenderPipelineId, FragmentState, RenderPipelineDescriptor,
    },
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CompareFunction,
            DepthStencilState, Extent3d, LoadOp, MultisampleState, Operations, PipelineCache,
            RenderPassDepthStencilAttachment, ShaderStages, StoreOp, Texture, TextureAspect,
            TextureDescriptor, TextureDimension, TextureUsages, TextureView, TextureViewDescriptor,
            binding_types::texture_depth_2d_multisampled,
        },
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
    },
};

use crate::{render::TERRAIN_DEPTH_FORMAT, shaders::DEPTH_COPY_SHADER};

#[derive(Component)]
pub struct TerrainViewDepthTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub depth_view: TextureView,
    pub stencil_view: TextureView,
    pub depth_copy_bind_group: BindGroup,
    size: UVec2,
    samples: u32,
}

impl TerrainViewDepthTexture {
    pub fn new(
        texture: CachedTexture,
        size: UVec2,
        samples: u32,
        device: &RenderDevice,
        depth_copy_pipeline: &DepthCopyPipeline,
    ) -> Self {
        let depth_view = texture.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::DepthOnly,
            ..default()
        });
        let stencil_view = texture.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::StencilOnly,
            ..default()
        });
        let depth_copy_bind_group = device.create_bind_group(
            None,
            &depth_copy_pipeline.layout,
            &BindGroupEntries::single(&depth_view),
        );

        Self {
            texture: texture.texture,
            view: texture.default_view,
            depth_view,
            stencil_view,
            depth_copy_bind_group,
            size,
            samples,
        }
    }

    pub fn get_attachment(&self) -> RenderPassDepthStencilAttachment<'_> {
        RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.0), // Clear depth
                store: StoreOp::Store,
            }),
            stencil_ops: Some(Operations {
                load: LoadOp::Clear(0), // Initialize stencil to 0 (lowest priority)
                store: StoreOp::Store,
            }),
        }
    }
}

pub fn prepare_terrain_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    depth_copy_pipeline: Res<DepthCopyPipeline>,
    views_3d: Query<(
        Entity,
        &ExtractedCamera,
        &Msaa,
        Option<&TerrainViewDepthTexture>,
    )>,
) {
    for (view, camera, msaa, existing) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };
        let samples = msaa.samples();

        if let Some(existing) = existing
            && existing.size == physical_target_size
            && existing.samples == samples
        {
            continue;
        }

        let descriptor = TextureDescriptor {
            label: Some("view_depth_texture"),
            size: Extent3d {
                depth_or_array_layers: 1,
                width: physical_target_size.x,
                height: physical_target_size.y,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: TextureDimension::D2,
            format: TERRAIN_DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let cached_texture = texture_cache.get(&device, descriptor);

        commands.entity(view).insert(TerrainViewDepthTexture::new(
            cached_texture,
            physical_target_size,
            samples,
            &device,
            &depth_copy_pipeline,
        ));
    }
}

#[derive(Resource)]
pub struct DepthCopyPipeline {
    pub layout: BindGroupLayout,
    pub id: CachedRenderPipelineId,
}

pub fn init_depth_copy_pipeline(
    mut commands: Commands,
    fullscreen_shader: Res<FullscreenShader>,
    device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    let entries = BindGroupLayoutEntries::sequential(
        ShaderStages::FRAGMENT,
        (texture_depth_2d_multisampled(),),
    );

    let layout = device.create_bind_group_layout(None, &entries);

    let id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: None,
        layout: vec![BindGroupLayoutDescriptor::new(
            "depth_copy_bind_group_layout",
            &entries,
        )],
        immediate_size: Default::default(),
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: asset_server.load(DEPTH_COPY_SHADER),
            shader_defs: vec![],
            entry_point: Some("fragment".into()),
            targets: vec![],
        }),
        primitive: Default::default(),
        depth_stencil: Some(DepthStencilState {
            format: CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: true.into(),
            depth_compare: CompareFunction::Always.into(),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState {
            count: 4, // Todo: specialize per camera ...
            ..Default::default()
        },
        zero_initialize_workgroup_memory: false,
    });

    commands.insert_resource(DepthCopyPipeline { layout, id });
}
