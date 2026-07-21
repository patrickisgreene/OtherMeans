use bevy::{
    material::descriptor::BindGroupLayoutDescriptor,
    render::render_resource::{
        BindGroupLayoutEntries, ShaderStages, StorageTextureAccess, TextureSampleType,
        binding_types::{texture_2d_array, texture_storage_2d_array, uniform_buffer},
    },
};

use crate::data::AttachmentFormat;

/// Hepler trait used to generate a mip layout from a type.
pub trait AsMipLayout {
    fn as_mip_layout(&self) -> BindGroupLayoutDescriptor;
}

impl AsMipLayout for AttachmentFormat {
    fn as_mip_layout(&self) -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "mip_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<u32>(false), // atlas_index
                    texture_2d_array(TextureSampleType::Float { filterable: true }), // parent
                    texture_storage_2d_array(
                        self.processing_format(),
                        StorageTextureAccess::WriteOnly,
                    ), // child
                ),
            ),
        )
    }
}
