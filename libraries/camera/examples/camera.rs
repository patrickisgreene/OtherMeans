use bevy::{
    input::common_conditions::input_toggle_active,
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use big_space::{plugin::BigSpaceMinimalPlugins, prelude::*};
use camera::{EarthCameraController, EarthCameraPlugin};

use workspace::EARTH_RADIUS;

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(None),
            EarthCameraPlugin,
            BigSpaceMinimalPlugins,
            WireframePlugin::default(),
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F10)),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn_big_space(Grid::default(), |parent| {
        parent.spawn_spatial((
            FloatingOrigin,
            Transform::from_translation(-Vec3::X * EARTH_RADIUS as f32 * 3.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            EarthCameraController::default(),
        ));

        parent.spawn_spatial((
            Name::new("Terrain"),
            Wireframe,
            Mesh3d(meshes.add(Sphere::new(EARTH_RADIUS as f32))),
            MeshMaterial3d(materials.add(StandardMaterial {
                unlit: true,
                base_color_texture: assets.load("textures/uv-checker.png").into(),
                ..default()
            })),
        ));
    });
}
