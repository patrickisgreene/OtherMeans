use std::time::Instant;

use airplanes::index::{build_coarse_index, coarse_ancestor, tile_xy};
use airplanes::routes::build_route_network;
use cities::descriptor::CitiesDatabase;
use terrain::math::Coordinate;
use terrain::prelude::{TerrainShape, TileCoordinate};
use workspace::lat_lon_to_unit_position;

fn load_cities() -> CitiesDatabase {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/earth/cities.ron");
    let content = std::fs::read_to_string(path).expect("read cities.ron");
    ron::de::from_str(&content).expect("parse cities.ron")
}

/// Sanity-checks that route generation and coarse-index building stay fast on the real cities
/// dataset - regression test for the freeze reported when zooming in, which was caused by
/// unbounded hub count (O(hubs^2) routes) and un-chunked multi-thousand-km route descriptors.
/// `network::chains_for_tile` itself (the other half of that freeze fix, and now also the piece
/// that samples real terrain elevation via `buildings::tile_height`) can't be exercised offline
/// here - it needs a live `&Image`/`&Attachment` from a loaded terrain, and
/// `terrain::data::attachment::Attachment` has no public constructor outside the `terrain` crate
/// (neither `automobiles` nor `roads` have offline tests of their own elevation-aware
/// `chains_for_tile` for the same reason). The `max_points`-per-chunk assertion below still
/// indirectly guards the chunking fix that made `chains_for_tile` fast.
#[test]
fn route_network_and_coarse_index_stay_fast() {
    let cities = load_cities();
    println!("cities: {}", cities.0.len());

    let start = Instant::now();
    let network = build_route_network(&cities);
    let build_elapsed = start.elapsed();
    println!(
        "routes: {} chunks (built in {:?})",
        network.0.len(),
        build_elapsed
    );
    assert!(
        build_elapsed.as_secs_f64() < 5.0,
        "route network build took {build_elapsed:?}, expected well under 5s"
    );

    let max_points = network.0.iter().map(|r| r.points.len()).max().unwrap_or(0);
    println!("max points per chunk: {max_points}");
    assert!(
        max_points <= 21,
        "a chunk had {max_points} points, expected chunking to bound it to ~20"
    );

    let shape = TerrainShape::WGS84;
    let target_lod = 5;

    let start = Instant::now();
    let coarse_index = build_coarse_index(&network, shape, target_lod);
    let index_elapsed = start.elapsed();
    println!(
        "coarse index: {} buckets at lod {} (built in {:?})",
        coarse_index.buckets.len(),
        coarse_index.coarse_lod,
        index_elapsed
    );
    assert!(
        index_elapsed.as_secs_f64() < 5.0,
        "coarse index build took {index_elapsed:?}, expected well under 5s"
    );

    let max_candidates = coarse_index
        .buckets
        .values()
        .map(|v| v.len())
        .max()
        .unwrap_or(0);
    println!("max candidates in a single coarse bucket: {max_candidates}");
}

/// Verifies Paris (a guaranteed hub - see `routes::HUB_COUNT` selection by rank/population) has
/// candidate routes registered in its own highest-LOD tile's coarse bucket - i.e. that route
/// generation and coarse indexing actually place data where a real city is, not just that *some*
/// chains exist somewhere on the globe. Stops short of calling `network::chains_for_tile` (see
/// `route_network_and_coarse_index_stay_fast`'s doc comment for why that needs a live terrain).
#[test]
fn paris_has_candidate_routes_in_its_own_tile() {
    const PARIS_LAT: f64 = 48.85809231626911;
    const PARIS_LON: f64 = 2.3529924615392135;

    let cities = load_cities();
    let network = build_route_network(&cities);

    let shape = TerrainShape::WGS84;
    let target_lod = 5;
    let coarse_index = build_coarse_index(&network, shape, target_lod);

    let paris_unit = lat_lon_to_unit_position(PARIS_LAT, PARIS_LON);
    let coordinate = Coordinate::from_unit_position(paris_unit, true);
    let tile_count = 2f64.powi(target_lod as i32);
    let xy = tile_xy(coordinate, tile_count);
    let paris_tile = TileCoordinate::new(coordinate.face, target_lod, xy);

    let ancestor = coarse_ancestor(paris_tile, coarse_index.coarse_lod);
    let candidates = coarse_index.buckets.get(&ancestor).expect(
        "Paris is a hub (see routes::HUB_COUNT selection by rank/population), so its own tile must have candidate routes",
    );
    println!(
        "Paris tile {paris_tile:?} has {} candidate routes",
        candidates.len()
    );
    assert!(!candidates.is_empty());
}
