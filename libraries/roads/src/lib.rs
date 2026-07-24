pub mod descriptor;
pub mod index;
pub mod preprocess;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

pub struct RoadsPlugin;

impl Plugin for RoadsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<descriptor::RoadNetwork>::new(&[
            "roads.ron",
        ]));
    }
}
