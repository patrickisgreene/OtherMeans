use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme},
    prelude::*,
};

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiTheme(create_dark_theme()));
    }
}
