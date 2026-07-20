use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

use crate::EarthParams;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct EarthMaterial {
    #[uniform(0)]
    pub render_mode: u32,
    #[texture(1)]
    #[sampler(2)]
    pub water_normal: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub water_normal_2: Handle<Image>,
    #[uniform(5)]
    pub constants: EarthParams,
}

impl EarthMaterial {
    pub fn new(asset_server: &AssetServer) -> Self {
        let water_normal = asset_server
            .load_builder()
            .with_settings(|s: &mut ImageLoaderSettings| {
                s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    mag_filter: bevy::image::ImageFilterMode::Linear,
                    min_filter: bevy::image::ImageFilterMode::Linear,
                    mipmap_filter: bevy::image::ImageFilterMode::Linear,
                    ..default()
                });
            })
            .load("textures/earth/water-normal.png");
        let water_normal_2 = asset_server
            .load_builder()
            .with_settings(|s: &mut ImageLoaderSettings| {
                s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    mag_filter: bevy::image::ImageFilterMode::Linear,
                    min_filter: bevy::image::ImageFilterMode::Linear,
                    mipmap_filter: bevy::image::ImageFilterMode::Linear,
                    ..default()
                });
            })
            .load("textures/earth/water-normal-2.png");
        Self {
            render_mode: 0,
            water_normal,
            water_normal_2,
            constants: EarthParams::default(),
        }
    }
}

impl Material for EarthMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/earth/fragment.wgsl".into()
    }
}
