use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::input::common_conditions::input_toggle_active;
use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use terrain::{prelude::*, TerrainConfigHandle, TerrainPlugins};

use workspace::EARTH_RADIUS;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct CustomMaterial {}

impl Material for CustomMaterial {}

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(None),
            TerrainPlugins::<CustomMaterial>::default(),
            shipping_lanes::ShippingLanesPlugin,
            shipping::ShippingPlugin,
            FpsOverlayPlugin::default(),
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F10)),
        ))
        .insert_resource(TerrainSettings::new(vec!["earth"]))
        .add_systems(Startup, initialize)
        .run();
}

fn initialize(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let material = materials.add(CustomMaterial {});

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
            OrbitalCameraController::default(),
        ));

        grid.spawn_spatial((
            MeshMaterial3d(material),
            TerrainConfigHandle(asset_server.load("earth/terrain.ron")),
        ));
    });
}
