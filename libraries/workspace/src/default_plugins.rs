use bevy::{app::PluginGroupBuilder, log::LogPlugin, prelude::*, window::PresentMode};

pub fn default_plugins(asset_dir: Option<String>) -> PluginGroupBuilder {
    let args = crate::ExampleCli::get();
    let file_path = match asset_dir {
        Some(path) => path,
        None => {
            let mut asset_folder = crate::workspace_dir();
            asset_folder.push("assets");
            asset_folder.to_string_lossy().to_string()
        }
    };
    DefaultPlugins
        .set(AssetPlugin {
            file_path,
            ..Default::default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        })
        .set(LogPlugin {
            level: args.log_level(),
            filter: args.log_filter(),
            ..default()
        })
}

pub fn default_plugins_big_space(asset_folder: Option<String>) -> PluginGroupBuilder {
    default_plugins(asset_folder).disable::<TransformPlugin>()
}
