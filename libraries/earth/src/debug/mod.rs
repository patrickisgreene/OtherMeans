use bevy::{
    feathers::{FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme},
    prelude::*,
};

mod components;
mod panel;
mod systems;
mod widgets;

pub use components::*;
pub use panel::*;
pub use widgets::*;

pub struct EarthDebugPlugin;

impl Plugin for EarthDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(create_dark_theme()))
            .add_systems(
                Update,
                (
                    systems::toggle_render_mode,
                    systems::sync_color_swatches,
                    systems::toggle_panel,
                ),
            );
    }
}
