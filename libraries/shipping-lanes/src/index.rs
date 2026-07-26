use bevy::{
    math::IVec2,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use terrain::math::Coordinate;
use terrain::prelude::{TerrainShape, TileCoordinate};
use workspace::lat_lon_to_unit_position;

use crate::descriptor::ShippingLaneNetwork;

/// Target real-world size (metres) of a `RoadCoarseIndex` bucket. The actual coarse LOD is
/// derived from this at build time (see `build_coarse_index`) relative to the terrain's own
/// `face_size`/`target_lod`, rather than being a fixed LOD constant - a fixed LOD would either be
/// finer than the terrain's own highest LOD on a shallow (low `lod_count`) config, which is
/// actively wrong (the coarse index would never be found when looking up an active tile's
/// ancestor, since an ancestor can't be *finer* than its descendant), or needlessly fine on a
/// deep one.
const COARSE_BUCKET_SIZE_M: f64 = 150_000.0;

/// Maps each coarse tile to the indices (into `RoadNetwork::0`) of roads that pass anywhere near
/// it. Built once, cheaply, from `network`'s raw vertices (no resampling, no elevation lookups) -
/// this is deliberately the only whole-globe structure any road-aware system (`ships`,
/// `buildings`) needs to build. Both crates narrow down to a small per-tile candidate list via
/// this index, then do their own (elevation-aware or not) fine-grained work only for that list.
#[derive(Resource, Default)]
pub struct ShippingLaneCoarseIndex {
    /// The LOD `buckets` keys are at - derived once from the terrain's own config at build time
    /// (see `build_coarse_index`), and must be used (via `coarse_ancestor`) for every lookup
    /// against `buckets`, since a fixed/assumed LOD could mismatch a differently-configured
    /// terrain.
    pub coarse_lod: u32,
    pub buckets: HashMap<TileCoordinate, Vec<u32>>,
}

/// `TerrainShape::is_spherical` is crate-private to `terrain`, so callers outside it re-derive the
/// same check.
pub fn shape_is_spherical(shape: TerrainShape) -> bool {
    !matches!(shape, TerrainShape::Plane { .. })
}

pub fn tile_xy(coordinate: Coordinate, tile_count: f64) -> IVec2 {
    (coordinate.uv * tile_count)
        .as_ivec2()
        .clamp(IVec2::ZERO, IVec2::splat(tile_count as i32 - 1))
}

/// Picks the coarse LOD `RoadCoarseIndex` should bucket at for a terrain whose highest active LOD
/// is `target_lod`: as close as possible to a `COARSE_BUCKET_SIZE_M`-sized bucket, but never finer
/// than `target_lod` itself (a bucket can't be an "ancestor" of something at the same or a coarser
/// level than it already is).
pub fn pick_coarse_lod(shape: TerrainShape, target_lod: u32) -> u32 {
    let desired_tile_count = (shape.face_size() / COARSE_BUCKET_SIZE_M).max(1.0);
    let desired_lod = desired_tile_count.log2().round().max(0.0) as u32;
    desired_lod.min(target_lod)
}

/// Builds `RoadCoarseIndex` from the whole `RoadNetwork`, once. Cheap: only looks at each road's
/// existing vertices (plus one midpoint per segment, to catch a straight segment briefly passing
/// through a coarse tile without a vertex landing inside it) - no resampling, no elevation.
pub fn build_coarse_index(
    network: &ShippingLaneNetwork,
    shape: TerrainShape,
    target_lod: u32,
) -> ShippingLaneCoarseIndex {
    let spherical = shape_is_spherical(shape);
    let coarse_lod = pick_coarse_lod(shape, target_lod);
    let tile_count = (coarse_lod as f64).exp2();

    let mut buckets: HashMap<TileCoordinate, Vec<u32>> = HashMap::new();

    for (road_index, road) in network.0.iter().enumerate() {
        if road.points.len() < 2 {
            continue;
        }

        let mut touched: HashSet<TileCoordinate> = HashSet::new();
        let mut sample = |lon: f64, lat: f64| {
            let unit = lat_lon_to_unit_position(lat, lon);
            let coordinate = Coordinate::from_unit_position(unit, spherical);
            let xy = tile_xy(coordinate, tile_count);
            touched.insert(TileCoordinate::new(coordinate.face, coarse_lod, xy));
        };

        for &(lon, lat) in &road.points {
            sample(lon, lat);
        }
        for pair in road.points.windows(2) {
            let (lon0, lat0) = pair[0];
            let (lon1, lat1) = pair[1];
            sample((lon0 + lon1) * 0.5, (lat0 + lat1) * 0.5);
        }

        for tile in touched {
            buckets.entry(tile).or_default().push(road_index as u32);
        }
    }

    ShippingLaneCoarseIndex {
        coarse_lod,
        buckets,
    }
}

/// Finds `tile`'s ancestor at `coarse_lod`, for looking it up in `RoadCoarseIndex::buckets`.
/// `coarse_lod` must be the same value the index was built with (`RoadCoarseIndex::coarse_lod`) -
/// `tile.lod` is always `>=` it by construction (see `pick_coarse_lod`).
pub fn coarse_ancestor(tile: TileCoordinate, coarse_lod: u32) -> TileCoordinate {
    let shift = tile.lod.saturating_sub(coarse_lod);
    TileCoordinate::new(tile.face, coarse_lod, tile.xy >> shift)
}

/// Builds a `grid_size x grid_size` occupancy grid (row-major, `[iy * grid_size + ix]`) flagging
/// which cells of `tile`'s placement grid a road passes through - the 2D, elevation-free
/// counterpart to `ships`' fine per-tile chain building. Cell `(ix, iy)` uses the exact same
/// UV convention as `buildings::instances::generate_tile_instances`'s placement grid, so callers
/// there can index it directly by their own `(ix, iy)` loop variables.
///
/// Unlike `ships`' path-following use case (which needs precise, elevation-aware waypoints),
/// this only needs a coarse "is a road anywhere in this cell" answer, so roads are resampled at a
/// step proportional to the cell size (not a fixed real-world distance) and elevation is ignored
/// entirely (`0.0` everywhere) - real-world segment length is only used to pick a safe step count.
pub fn tile_road_occupancy(
    tile: TileCoordinate,
    network: &ShippingLaneNetwork,
    candidates: &[u32],
    shape: TerrainShape,
    grid_size: u32,
) -> Vec<bool> {
    let spherical = shape_is_spherical(shape);
    let tile_count = 2f64.powi(tile.lod as i32);
    let cell_size_m = shape.face_size() / tile_count / grid_size as f64;
    let step_m = (cell_size_m * 0.5).max(1.0);

    let mut occupancy = vec![false; (grid_size * grid_size) as usize];

    let mut mark = |lon: f64, lat: f64| {
        let unit = lat_lon_to_unit_position(lat, lon);
        let coordinate = Coordinate::from_unit_position(unit, spherical);
        if coordinate.face != tile.face {
            return;
        }

        let local_uv = coordinate.uv * tile_count - tile.xy.as_dvec2();
        if local_uv.x < 0.0 || local_uv.x >= 1.0 || local_uv.y < 0.0 || local_uv.y >= 1.0 {
            return;
        }

        let ix = ((local_uv.x * grid_size as f64) as u32).min(grid_size - 1);
        let iy = ((local_uv.y * grid_size as f64) as u32).min(grid_size - 1);
        occupancy[(iy * grid_size + ix) as usize] = true;
    };

    for &road_index in candidates {
        let Some(road) = network.0.get(road_index as usize) else {
            continue;
        };
        if road.points.is_empty() {
            continue;
        }

        let (mut lon0, mut lat0) = road.points[0];
        mark(lon0, lat0);

        for &(lon1, lat1) in &road.points[1..] {
            let world0 = shape.position_unit_to_local(lat_lon_to_unit_position(lat0, lon0), 0.0);
            let world1 = shape.position_unit_to_local(lat_lon_to_unit_position(lat1, lon1), 0.0);
            let segment_length = (world1 - world0).length();
            let steps = ((segment_length / step_m).ceil() as u32).max(1);

            for step in 1..=steps {
                let t = step as f64 / steps as f64;
                mark(lon0 + (lon1 - lon0) * t, lat0 + (lat1 - lat0) * t);
            }

            lon0 = lon1;
            lat0 = lat1;
        }
    }

    occupancy
}
