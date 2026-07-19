use bevy::{prelude::*, shader::ShaderDefVal};

use crate::data::AttachmentFormat;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MipPipelineKey {
    pub(crate) format: AttachmentFormat,
}

impl MipPipelineKey {
    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        let format = match self.format {
            AttachmentFormat::Rgb8U => "RGB8U",
            AttachmentFormat::Rgba8U => "RGBA8U",
            AttachmentFormat::R16U => "R16U",
            AttachmentFormat::R16I => "R16I",
            AttachmentFormat::Rg16U => "RG16U",
            AttachmentFormat::R32F => "R32F",
        };

        shader_defs.push(format.into());

        shader_defs
    }
}
