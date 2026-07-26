use bevy::prelude::*;
use big_space::prelude::*;
use cities::descriptor::CityLightsDatabase;
use cities::lighting::{CityLightCluster, CityLightsRoot};
use terrain::math::TerrainShape;
use workspace::lat_lon_to_unit_position;

use crate::{Earth, GameCityLightsDatabase};

// Placeholder brightness/range curve, scaled by cluster population/size - the world uses
// real-world meters, so these need visual tuning once lights are on screen.
const BASE_INTENSITY: f32 = 5_000_000_000.0;
const MAX_INTENSITY: f32 = 5_000_000_000_000.0;
const BASE_RANGE: f32 = 200_000.0;
const RANGE_PER_CITY: f32 = 200000.0;
const MAX_RANGE: f32 = 2_000_000.0;

// Height above the WGS84 ellipsoid the light sits at. Cities spawn with height 0.0 (ellipsoid
// surface), but real terrain elevation is almost always above that - a light sitting at/under the
// actual rendered surface contributes zero PBR lighting (N dot L <= 0), no matter how bright.
// This clears real-world terrain (Everest is ~8,849m) with margin.
const LIGHT_HEIGHT: f64 = 10_000.0;

pub fn spawn_city_lights(
    mut commands: Commands,
    light_clusters: Res<Assets<CityLightsDatabase>>,
    light_db: Res<GameCityLightsDatabase>,
    earth: Query<Entity, With<Earth>>,
    grids: Grids,
) {
    let db = light_clusters.get(&light_db.0).unwrap();

    let grid_entity = grids.parent_grid_entity(earth.single().unwrap()).unwrap();
    let grid = grids.get(grid_entity);

    let root = commands
        .spawn((
            Name::new("City Lights"),
            CityLightsRoot,
            Grid::default(),
            Transform::default(),
            Visibility::default(),
            CellCoord::default(),
            ChildOf(grid_entity),
        ))
        .id();

    for cluster in &db.0 {
        let unit = lat_lon_to_unit_position(cluster.lat, cluster.lon);
        let local_position = TerrainShape::WGS84.position_unit_to_local(unit, LIGHT_HEIGHT);
        let (cell, translation) = grid.translation_to_grid(local_position);

        let intensity =
            (BASE_INTENSITY * (cluster.population.max(1) as f32).sqrt()).min(MAX_INTENSITY) * 100.0;
        let range =
            (BASE_RANGE + RANGE_PER_CITY * cluster.city_count as f32).min(MAX_RANGE) * 1000.0;

        commands.spawn((
            Name::new(format!(
                "City Light Cluster ({} cities)",
                cluster.city_count
            )),
            CityLightCluster {
                population: cluster.population,
                city_count: cluster.city_count,
            },
            PointLight {
                intensity,
                range,
                radius: 0.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(translation),
            Visibility::Hidden,
            cell,
            ChildOf(root),
        ));
    }
}
