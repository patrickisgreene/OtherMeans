use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

use crate::descriptor::CityLightsDatabase;
use crate::lighting::systems::update_city_light_visibility;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<CityLightsDatabase>::new(&[
            "city-lights.ron",
        ]))
        .add_systems(Update, update_city_light_visibility);
    }
}
