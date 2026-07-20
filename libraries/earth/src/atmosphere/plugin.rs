use bevy::{
    core_pipeline::{Core3dSystems, schedule::Core3d, tonemapping::tonemapping},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
    },
};

use super::{
    EarthAtmosphereSettings, pass::post_process_pass, pipeline::EarthAtmospherePipeline,
    systems::update_post_process_settings,
};

/// It is generally encouraged to set up post processing effects as a plugin
pub struct EarthAtmospherePlugin;

impl Plugin for EarthAtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
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
