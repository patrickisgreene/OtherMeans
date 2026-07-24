use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use earth::EarthMaterial;
use terrain::prelude::*;
use workspace::EARTH_RADIUS;

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(None),
            earth::EarthPlugin,
            earth::debug::EarthDebugPlugin,
            buildings::BuildingsPlugin,
            roads::RoadsPlugin,
            vehicles::VehiclesPlugin,
            FpsOverlayPlugin::default(),
        ))
        .add_systems(Update, initialize)
        .run();
}

#[allow(clippy::too_many_arguments)]
fn initialize(
    mut completed: Local<bool>,
    mut config: Local<Option<Handle<TerrainConfig>>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<TerrainSettings>,
    configs: Res<Assets<TerrainConfig>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<EarthMaterial>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
) {
    if *completed {
        return;
    }
    if config.is_none() {
        *config = Some(asset_server.load("earth/terrain.ron"));
        return;
    }
    let handle = config.clone().unwrap();
    let Some(config) = configs.get(&handle) else {
        return;
    };

    let mut view = Entity::PLACEHOLDER;
    let mut root = Entity::PLACEHOLDER;

    commands.spawn_big_space(Grid::default(), |parent| {
        view = parent
            .spawn_spatial((
                Transform::from_translation(-Vec3::X * EARTH_RADIUS as f32 * 3.0)
                    .looking_to(Vec3::X, Vec3::Y),
                AmbientLight {
                    brightness: 100.0,
                    ..default()
                },
                earth::EarthAtmosphereSettings::default(),
                OrbitalCameraController::default(),
            ))
            .id();

        root = parent
            .spawn_grid_default((Name::new("Terrain"), InheritedVisibility::default()))
            .id();
    });

    let material = materials.add(EarthMaterial::new(&asset_server));

    let earth = commands
        .spawn((
            ChildOf(root),
            config.shape.transform(),
            TileAtlas::new(&config, &mut buffers, &*settings),
            MeshMaterial3d(material),
        ))
        .id();

    tile_trees.insert(
        (earth, view),
        TileTree::new(
            &config,
            &TerrainViewConfig {
                viewport: TerrainViewport::Square,
                ..default()
            },
            (earth, view),
            &mut commands,
            &mut buffers,
        ),
    );

    *completed = true;
}
