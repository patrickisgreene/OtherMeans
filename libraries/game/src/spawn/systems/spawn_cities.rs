use bevy::prelude::*;
use big_space::prelude::*;
use metrics::Metric;
use scenario::Scenario;
use terrain::math::TerrainShape;
use workspace::lat_lon_to_unit_position;

use crate::{Earth, GameCitiesDatabase, GameScenario};
use cities::{CitiesRoot, CityFilter, CityLabelRank, CityScaleRank, descriptor::CitiesDatabase};

pub fn spawn_cities(
    mut commands: Commands,
    cities: Res<Assets<CitiesDatabase>>,
    city_db: Res<GameCitiesDatabase>,
    scenario: Res<GameScenario>,
    scenarios: Res<Assets<Scenario>>,
    earth: Query<Entity, With<Earth>>,
    grids: Grids,
) {
    let db = cities.get(&city_db.0).unwrap();
    let scenario = scenarios.get(&scenario.0).unwrap();

    let grid_entity = grids.parent_grid_entity(earth.single().unwrap()).unwrap();
    let grid = grids.get(grid_entity);

    let cities_root = commands
        .spawn((
            Name::new("Cities"),
            CitiesRoot,
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
                CityFilter::MinPopulation(min_pop) => {
                    if city.population < *min_pop {
                        continue 'city;
                    }
                }
            }
        }

        let unit = lat_lon_to_unit_position(city.lat, city.lon);
        let local_position = TerrainShape::WGS84.position_unit_to_local(unit, 0.0);
        let (cell, translation) = grid.translation_to_grid(local_position);

        let combatant_id = scenario.combatant_for_place_or_country(city.id, &city.country);

        let entity = commands
            .spawn((
                Name::new(city.name.to_string()),
                Metric::<"damage">::new(0),
                Metric::<"activity">::new(0),
                Metric::<"unrest">::new(0),
                CityLabelRank(city.label_rank),
                CityScaleRank(city.scale_rank),
                city.industries,
                Transform::from_translation(translation),
                Visibility::default(),
                cell,
                ChildOf(cities_root),
            ))
            .id();

        if let Some(combatant) = combatant_id {
            commands.entity(entity).insert(combatant);
        }
    }
}
