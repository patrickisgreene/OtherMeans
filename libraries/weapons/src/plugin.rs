use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

use crate::data::Weapon;

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<Weapon>::new(&["weapon.ron"]))
            .add_systems(Update, super::systems::advance_strike_mission);
    }
}
