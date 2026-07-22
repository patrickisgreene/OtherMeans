pub mod descriptor;
pub mod preprocess;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

pub struct CitiesPlugin;

impl Plugin for CitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<descriptor::CitiesDatabase>::new(&[
            "cities.ron",
        ]));
    }
}
