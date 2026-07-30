use bevy::{
    core_pipeline::{Core3dSystems, schedule::Core3d, tonemapping::tonemapping},
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
        extract_resource::ExtractResourcePlugin,
    },
};

use super::{
    EarthAtmosphereSettings, pass::post_process_pass,
    pipeline::{EarthAtmospherePipeline, WindTexture},
    systems::update_post_process_settings,
};

/// Loads the small wind-flow texture (see `WindTexture`'s doc comment) once at startup. Wraps
/// (repeats) over longitude, clamps over latitude - it's an equirectangular map, so only the
/// horizontal axis is cyclic.
fn load_wind_texture(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            // The R/G channels encode a wind direction vector, not real color - loading it
            // sRGB would corrupt those values the same way it would on earth_attachment's ocean-side
            // data channels (see undo_srgb in fragment.wgsl); avoid that by loading linear.
            s.is_srgb = false;
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::ClampToEdge,
                mag_filter: bevy::image::ImageFilterMode::Linear,
                min_filter: bevy::image::ImageFilterMode::Linear,
                mipmap_filter: bevy::image::ImageFilterMode::Linear,
                ..default()
            });
        })
        .load("textures/earth/wind-flow.png");
    commands.insert_resource(WindTexture(handle));
}

/// It is generally encouraged to set up post processing effects as a plugin
pub struct EarthAtmospherePlugin;

impl Plugin for EarthAtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_wind_texture)
            .add_plugins((
                // The settings will be a component that lives in the main world but will
                // be extracted to the render world every frame.
                // This makes it possible to control the effect from the main world.
                // This plugin will take care of extracting it automatically.
                // It's important to derive [`ExtractComponent`] on [`PostProcessingSettings`]
                // for this plugin to work correctly.
                ExtractComponentPlugin::<EarthAtmosphereSettings>::default(),
                // The settings will also be the data used in the shader.
                // This plugin will prepare the component for the GPU by creating a uniform buffer
                // and writing the data to that buffer every frame.
                UniformComponentPlugin::<EarthAtmosphereSettings>::default(),
                // WindTexture is loaded once and never changes - this just clones the Handle<Image>
                // into the render world every frame, which is cheap.
                ExtractResourcePlugin::<WindTexture>::default(),
            ))
            .add_systems(PostUpdate, update_post_process_settings);

        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // RenderStartup runs once on startup after all plugins are built
        // It is useful to initialize data that will only live in the RenderApp
        render_app.add_systems(RenderStartup, EarthAtmospherePipeline::initialize);

        // Bevy's renderer used to be a render graph of Nodes wired together with explicit
        // edges; that's been replaced with camera-driven schedules (Core3d/Core2d/etc, see
        // bevy_core_pipeline::schedule) containing plain systems ordered relative to each
        // other via system sets - terrain's own `terrain_pass` (libraries/terrain/src/render/
        // pass/mod.rs) already follows this pattern.
        //
        // `tonemapping` runs in `Core3dSystems::PostProcess`, with `upscaling` ordered after
        // the whole set - putting our system in that same set, explicitly after `tonemapping`,
        // reproduces the old "runs between Tonemapping and EndMainPassPostProcessing" position
        // (still before the final upscale/blit, and before EguiPass which runs after that).
        render_app.add_systems(
            Core3d,
            post_process_pass
                .after(tonemapping)
                .in_set(Core3dSystems::PostProcess),
        );
    }
}
