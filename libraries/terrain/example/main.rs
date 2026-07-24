use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::shader::ShaderRef;
use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use terrain::{TerrainConfigHandle, TerrainPlugins, prelude::*};

use workspace::EARTH_RADIUS;

#[derive(ShaderType, Clone)]
struct GradientInfo {
    mode: u32,
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct CustomMaterial {
    #[texture(0)]
    #[sampler(1)]
    gradient: Handle<Image>,
    #[uniform(2)]
    gradient_info: GradientInfo,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(Some("example/assets".into())),
            TerrainPlugins::<CustomMaterial>::default(),
            FpsOverlayPlugin::default(),
        ))
        .insert_resource(TerrainSettings::new(vec!["albedo"]))
        .add_systems(Startup, initialize)
        .run();
}

fn initialize(
    mut commands: Commands,
    mut images: ResMut<LoadingImages>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let gradient1 = asset_server.load("textures/gradient1.png");
    images.load_image(
        &gradient1,
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
    );

    let material = materials.add(CustomMaterial {
        gradient: gradient1,
        gradient_info: GradientInfo { mode: 2 },
    });

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
            TerrainConfigHandle(asset_server.load("terrain/terrain.ron")),
        ));
    });
}
