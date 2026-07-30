use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::input::common_conditions::input_toggle_active;
use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use big_space::prelude::*;
use cities::descriptor::CitiesDatabase;
use scenario::Scenario;
use terrain::{TerrainConfigHandle, TerrainPlugins, prelude::*};

use workspace::{EARTH_RADIUS, lat_lon_to_unit_position};

#[derive(States, Debug, Default, PartialEq, Clone, Copy, Hash, Eq)]
struct ExampleState;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct CustomMaterial {}

impl Material for CustomMaterial {}

fn main() {
    App::new()
        .add_plugins((
            workspace::default_plugins_big_space(None),
            TerrainPlugins::<CustomMaterial>::default(),
            cities::CitiesPlugin,
            FeathersPlugins,
            labels::LabelsPlugin::new(ExampleState),
            FpsOverlayPlugin::default(),
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F10)),
        ))
        .init_state::<ExampleState>()
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(TerrainSettings::new(vec!["earth"]))
        .add_systems(Startup, initialize)
        .add_systems(Update, spawn_cities)
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

pub fn spawn_cities(
    mut spawned: Local<bool>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    cities: Res<Assets<CitiesDatabase>>,
    mut city_db: Local<Option<Handle<CitiesDatabase>>>,
    earth: Query<Entity, With<TerrainConfigHandle>>,
    grids: Grids,
) {
    if *spawned {
        return;
    }
    if city_db.is_none() {
        *city_db = Some(assets.load("earth/cities.ron"));
        return;
    } else if let Some(handle) = &*city_db {
        match cities.get(handle) {
            Some(_) => {}
            None => return,
        }
    }
    let db = cities.get(city_db.as_ref().unwrap()).unwrap();
    let scenario = Scenario::players_america();

    let grid_entity = grids.parent_grid_entity(earth.single().unwrap()).unwrap();
    let grid = grids.get(grid_entity);

    let cities_root = commands
        .spawn((
            Name::new("Cities"),
            cities::CitiesRoot,
            Grid::default(),
            Transform::default(),
            Visibility::default(),
            CellCoord::default(),
            ChildOf(grid_entity),
        ))
        .id();

    'city: for city in &db.0 {
        for filter in &scenario.city_filters {
            match filter {
                cities::CityFilter::MinPopulation(min_pop) => {
                    if city.population < *min_pop {
                        continue 'city;
                    }
                }
            }
        }

        let unit = lat_lon_to_unit_position(city.lat, city.lon);
        let local_position = TerrainShape::WGS84.position_unit_to_local(unit, 0.0);
        let (cell, translation) = grid.translation_to_grid(local_position);

        commands.spawn((
            Name::new(city.name.to_string()),
            cities::CityLabelRank(city.label_rank),
            cities::CityScaleRank(city.scale_rank),
            city.industries,
            Transform::from_translation(translation),
            Visibility::default(),
            cell,
            ChildOf(cities_root),
        ));
    }

    *spawned = true;
}
