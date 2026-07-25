use bevy::prelude::*;
use bevy::{
    dev_tools::fps_overlay::FpsOverlayPlugin, input::common_conditions::input_toggle_active,
};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use earth::EarthMaterial;
use terrain::{TerrainConfigHandle, prelude::*};
use workspace::EARTH_RADIUS;

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(None),
            earth::EarthPlugin,
            earth::debug::EarthDebugPlugin,
            FpsOverlayPlugin::default(),
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F10)),
        ))
        .add_systems(Startup, initialize)
        .run();
}

fn initialize(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<EarthMaterial>>,
) {
    let material = materials.add(EarthMaterial::new(&asset_server));

    commands.spawn_big_space(Grid::default(), |grid| {
        grid.insert(InheritedVisibility::default());
        grid.spawn_spatial((
            Transform::from_translation(-Vec3::X * EARTH_RADIUS as f32 * 3.0)
                .looking_to(Vec3::X, Vec3::Y),
            AmbientLight {
                brightness: 100.0,
                ..default()
            },
            TerrainViewConfig::default(),
            earth::EarthAtmosphereSettings::default(),
            OrbitalCameraController::default(),
        ));

        grid.spawn_spatial((
            MeshMaterial3d(material),
            TerrainConfigHandle(asset_server.load("earth/terrain.ron")),
        ));
    });
}
