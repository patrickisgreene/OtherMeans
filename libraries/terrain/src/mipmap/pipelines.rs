use bevy::{
    material::descriptor::{BindGroupLayoutDescriptor, ComputePipelineDescriptor},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::SpecializedComputePipeline,
};
use strum::IntoEnumIterator;

use crate::{
    mipmap::{MipPipelineKey, create_mip_layout},
    shaders::MIP_SHADER,
    data::AttachmentFormat,
};

#[derive(Resource)]
pub struct MipPipelines {
    pub(crate) mip_layouts: HashMap<AttachmentFormat, BindGroupLayoutDescriptor>,
    mip_shader: Handle<Shader>,
}

impl FromWorld for MipPipelines {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let mip_layouts = AttachmentFormat::iter()
            .map(|format| (format, create_mip_layout(format)))
            .collect();
        let mip_shader = asset_server.load(MIP_SHADER);

        Self {
            mip_layouts,
            mip_shader,
        }
    }
}

impl SpecializedComputePipeline for MipPipelines {
    type Key = MipPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("mip_pipeline".into()),
            layout: vec![self.mip_layouts[&key.format].clone()],
            //push_constant_ranges: default(),
            shader: self.mip_shader.clone(),
            shader_defs: key.shader_defs(),
            entry_point: Some("main".into()),
            zero_initialize_workgroup_memory: false,
            immediate_size: Default::default(),
        }
    }
}
