use bevy::{
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
};
use big_space::{plugin::BigSpaceMinimalPlugins, prelude::*};
use camera::{EarthCameraController, EarthCameraPlugin};

use workspace::EARTH_RADIUS;

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(Some("assets".into())),
            EarthCameraPlugin,
            BigSpaceMinimalPlugins,
            WireframePlugin::default(),
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
                base_color_texture: assets.load("uv-checker.png").into(),
                ..default()
            })),
        ));
    });
}
