use bevy::{
    material::descriptor::{BindGroupLayoutDescriptor, ComputePipelineDescriptor},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::SpecializedComputePipeline,
};

use crate::{
    data::AttachmentFormat,
    mipmap::{AsMipLayout, MIP_SHADER, MipPipelineKey},
};

#[derive(Resource)]
pub struct MipPipelines {
    pub(crate) mip_layouts: HashMap<AttachmentFormat, BindGroupLayoutDescriptor>,
    mip_shader: Handle<Shader>,
}

pub(crate) fn init_mip_pipelines(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mip_layouts = AttachmentFormat::ALL
        .into_iter()
        .map(|format| (format, format.as_mip_layout()))
        .collect();
    let mip_shader = asset_server.load(MIP_SHADER);

    commands.insert_resource(MipPipelines {
        mip_layouts,
        mip_shader,
    });
}

impl SpecializedComputePipeline for MipPipelines {
    type Key = MipPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("mip_pipeline".into()),
            layout: vec![self.mip_layouts[&key.format].clone()],
            shader: self.mip_shader.clone(),
            shader_defs: key.shader_defs(),
            entry_point: Some("main".into()),
            zero_initialize_workgroup_memory: false,
            immediate_size: Default::default(),
        }
    }
}
