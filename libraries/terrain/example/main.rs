use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use terrain::{TerrainPlugins, prelude::*};

const RADIUS: f64 = 6371000.0;

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
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "example/assets".into(),
                    ..default()
                })
                .build()
                .disable::<TransformPlugin>(),
            TerrainPlugins::<CustomMaterial>::default(),
            FpsOverlayPlugin::default(),
        ))
        .insert_resource(TerrainSettings::new(vec!["albedo"]))
        .add_systems(Update, initialize)
        .run();
}

struct MarsConfig(Handle<TerrainConfig>);

fn initialize(
    mut spawned: Local<bool>,
    mut config: Local<Option<MarsConfig>>,
    mut commands: Commands,
    settings: Res<TerrainSettings>,
    configs: Res<Assets<TerrainConfig>>,
    mut images: ResMut<LoadingImages>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
) {
    if *spawned {
        return;
    }

    let config = if let Some(config) = &*config {
        if let Some(a) = configs.get(&config.0) {
            a
        } else {
            return;
        }
    } else {
        *config = Some(MarsConfig(asset_server.load("terrain/config.tc.ron")));
        return;
    };

    let gradient1 = asset_server.load("textures/gradient1.png");
    images.load_image(
        &gradient1,
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
    );

    let mut view = Entity::PLACEHOLDER;
    let mut root = Entity::PLACEHOLDER;

    commands.spawn_big_space(Grid::default(), |grid| {
        root = grid.id();
        view = grid
            .spawn_spatial((
                Transform::from_translation(-Vec3::X * RADIUS as f32 * 3.0)
                    .looking_to(Vec3::X, Vec3::Y),
                AmbientLight {
                    brightness: 100.0,
                    ..default()
                },
                OrbitalCameraController::default(),
            ))
            .id();
    });

    let terrain = commands
        .spawn((
            config.shape.transform(),
            TileAtlas::new(&config, &mut buffers, &settings),
            MeshMaterial3d(materials.add(CustomMaterial {
                gradient: gradient1,
                gradient_info: GradientInfo { mode: 2 },
            })),
        ))
        .id();

    commands.entity(root).add_child(terrain);

    tile_trees.insert(
        (terrain, view),
        TileTree::new(
            &config,
            &TerrainViewConfig::default(),
            (terrain, view),
            &mut commands,
            &mut buffers,
        ),
    );

    *spawned = true;
}
