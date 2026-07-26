use bevy::{
    math::IVec2,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use terrain::math::Coordinate;
use terrain::prelude::{TerrainShape, TileCoordinate};
use workspace::lat_lon_to_unit_position;

use crate::routes::AirplaneRouteNetwork;

/// Target real-world size (metres) of an `AirplaneRouteCoarseIndex` bucket. Copy of
/// `shipping_lanes::index`'s `COARSE_BUCKET_SIZE_M` - the same "derive the coarse LOD from the
/// terrain's own config, never finer than the terrain's highest active LOD" reasoning applies
/// unchanged to flight routes.
const COARSE_BUCKET_SIZE_M: f64 = 150_000.0;

/// Maps each coarse tile to the indices (into `AirplaneRouteNetwork::0`) of routes that pass
/// anywhere near it. Built once, cheaply, from `network`'s already-fine (~50km-spaced, see
/// `routes::GREAT_CIRCLE_SAMPLE_STEP_M`) points - no further resampling, no elevation lookups.
#[derive(Resource, Default)]
pub struct AirplaneRouteCoarseIndex {
    /// The LOD `buckets` keys are at - derived once from the terrain's own config at build time
    /// (see `build_coarse_index`), and must be used (via `coarse_ancestor`) for every lookup.
    pub coarse_lod: u32,
    pub buckets: HashMap<TileCoordinate, Vec<u32>>,
}

/// `TerrainShape::is_spherical` is crate-private to `terrain`, so callers outside it re-derive the
/// same check. Copy of `shipping_lanes::index::shape_is_spherical`.
pub fn shape_is_spherical(shape: TerrainShape) -> bool {
    !matches!(shape, TerrainShape::Plane { .. })
}

pub fn tile_xy(coordinate: Coordinate, tile_count: f64) -> IVec2 {
    (coordinate.uv * tile_count)
        .as_ivec2()
        .clamp(IVec2::ZERO, IVec2::splat(tile_count as i32 - 1))
}

/// Picks the coarse LOD to bucket at for a terrain whose highest active LOD is `target_lod`: as
/// close as possible to a `COARSE_BUCKET_SIZE_M`-sized bucket, but never finer than `target_lod`
/// itself.
pub fn pick_coarse_lod(shape: TerrainShape, target_lod: u32) -> u32 {
    let desired_tile_count = (shape.face_size() / COARSE_BUCKET_SIZE_M).max(1.0);
    let desired_lod = desired_tile_count.log2().round().max(0.0) as u32;
    desired_lod.min(target_lod)
}

/// Builds `AirplaneRouteCoarseIndex` from the whole `AirplaneRouteNetwork`, once. Cheap: only
/// looks at each route's existing points (plus one midpoint per segment, to catch a segment
/// briefly passing through a coarse tile without a point landing inside it).
pub fn build_coarse_index(
    network: &AirplaneRouteNetwork,
    shape: TerrainShape,
    target_lod: u32,
) -> AirplaneRouteCoarseIndex {
    let spherical = shape_is_spherical(shape);
    let coarse_lod = pick_coarse_lod(shape, target_lod);
    let tile_count = (coarse_lod as f64).exp2();

    let mut buckets: HashMap<TileCoordinate, Vec<u32>> = HashMap::new();

    for (route_index, route) in network.0.iter().enumerate() {
        if route.points.len() < 2 {
            continue;
        }

        let mut touched: HashSet<TileCoordinate> = HashSet::new();
        let mut sample = |lon: f64, lat: f64| {
            let unit = lat_lon_to_unit_position(lat, lon);
            let coordinate = Coordinate::from_unit_position(unit, spherical);
            let xy = tile_xy(coordinate, tile_count);
            touched.insert(TileCoordinate::new(coordinate.face, coarse_lod, xy));
        };

        for &(lon, lat) in &route.points {
            sample(lon, lat);
        }
        for pair in route.points.windows(2) {
            let (lon0, lat0) = pair[0];
            let (lon1, lat1) = pair[1];
            sample((lon0 + lon1) * 0.5, (lat0 + lat1) * 0.5);
        }

        for tile in touched {
            buckets.entry(tile).or_default().push(route_index as u32);
        }
    }

    AirplaneRouteCoarseIndex {
        coarse_lod,
        buckets,
    }
}

/// Finds `tile`'s ancestor at `coarse_lod`, for looking it up in
/// `AirplaneRouteCoarseIndex::buckets`. `coarse_lod` must be the same value the index was built
/// with (`AirplaneRouteCoarseIndex::coarse_lod`) - `tile.lod` is always `>=` it by construction
/// (see `pick_coarse_lod`).
pub fn coarse_ancestor(tile: TileCoordinate, coarse_lod: u32) -> TileCoordinate {
    let shift = tile.lod.saturating_sub(coarse_lod);
    TileCoordinate::new(tile.face, coarse_lod, tile.xy >> shift)
}
