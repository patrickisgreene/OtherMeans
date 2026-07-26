use bevy::{image::Image, math::DVec3, prelude::*};
use terrain::data::attachment::Attachment;
use terrain::math::Coordinate;
use terrain::prelude::{TerrainShape, TileCoordinate};

use roads::index::{shape_is_spherical, tile_xy};
use workspace::lat_lon_to_unit_position;

use buildings::tile_height::{sample_height_tile, tile_local_uv};

/// Real-world spacing (metres) at which raw shapefile road segments are resampled when building
/// the fine, tile-local chains for one active tile. `ne_10m_roads.shp` is a generalized 1:10M
/// dataset - a single segment between two shapefile vertices can be tens or hundreds of km long,
/// far bigger than a highest-LOD terrain tile - so segments are walked in small steps and
/// re-bucketed per step rather than assigned to a single tile as a whole.
const ROAD_SAMPLE_STEP_M: f64 = 300.0;

/// Maximum number of waypoints stored per in-tile road chain. Bounds the size of the per-automobile
/// waypoint array the vertex shader walks (see `render::pipeline`); chains longer than this in
/// raw sample count are split into multiple chains rather than growing that array unboundedly.
pub const MAX_WAYPOINTS: usize = 8;

/// A single contiguous, tile-local sequence of points a automobile can loop along. Always wholly
/// contained within one terrain tile, so a automobile bound to a chain never needs to hand off to a
/// neighbouring tile mid-drive.
#[derive(Clone)]
pub struct RoadChain {
    /// Waypoint positions, relative to the owning tile's center (same convention as
    /// `buildings::instances::InstanceData::position_and_footprint`).
    pub waypoints: Vec<Vec3>,
    /// Cumulative real-world distance (metres) from `waypoints[0]` to each waypoint; same
    /// length as `waypoints`, `cumulative[0] == 0.0`.
    pub cumulative: Vec<f32>,
    /// Surface normal at the chain's first waypoint, used as a constant "up" for every automobile
    /// on this chain. A tile at the highest LOD is small enough that the normal barely varies
    /// across it, so one normal per chain (rather than per-waypoint) is an acceptable
    /// simplification for a purely visual effect.
    pub normal: Vec3,
}

impl RoadChain {
    pub fn total_length(&self) -> f32 {
        self.cumulative.last().copied().unwrap_or(0.0)
    }
}

/// A (lon, lat) point resolved to `target_lod`'s tile grid, plus its real world-space position
/// (already including terrain elevation, matching how `buildings` places instances) and its unit
/// cube-sphere position (used to derive a surface normal). Elevation-aware, unlike
/// `roads::index`'s coarse bucketing - this is why it stays in `automobiles` rather than moving there
/// too (see `libraries/roads/src/index.rs`'s doc comment on `tile_road_occupancy`).
struct Located {
    tile: TileCoordinate,
    world: DVec3,
    unit: DVec3,
}

#[allow(clippy::too_many_arguments)]
fn locate(
    lon_deg: f64,
    lat_deg: f64,
    shape: TerrainShape,
    spherical: bool,
    height_image: &Image,
    height_attachment: &Attachment,
    height_scale: f32,
    target: TileCoordinate,
    tile_count: f64,
) -> Located {
    let unit = lat_lon_to_unit_position(lat_deg, lon_deg);
    let coordinate = Coordinate::from_unit_position(unit, spherical);

    // Sampled against `target`'s own height tile even for candidate points that turn out to
    // fall outside it (an out-of-range UV just clamps to that tile's edge) - those points get
    // discarded by `ChainBuilder::push`'s `in_target` check regardless, so their elevation only
    // has to be plausible enough to not distort segment-length/step-count estimates, not exact.
    let local_uv = tile_local_uv(coordinate.uv, target);
    let elevation = sample_height_tile(height_image, height_attachment, local_uv) * height_scale;
    let world = shape.position_unit_to_local(unit, elevation as f64);

    let xy = tile_xy(coordinate, tile_count);

    Located {
        tile: TileCoordinate::new(coordinate.face, target.lod, xy),
        world,
        unit,
    }
}

/// Accumulates consecutive samples that fall inside `target` into `RoadChain`s, discarding any
/// run of samples that falls outside it. Used by `chains_for_tile`, which only ever cares about
/// one tile at a time (unlike a whole-globe builder, this never needs to key output by tile - the
/// caller already knows which tile it's building for).
struct ChainBuilder {
    target: TileCoordinate,
    active: bool,
    points: Vec<DVec3>,
    /// Unit cube-sphere position of `points[0]`, kept around solely to derive `RoadChain::normal`
    /// once the chain is flushed.
    first_unit: Option<DVec3>,
    /// Unit cube-sphere position of the most recently pushed point, kept around so a carried-over
    /// point (see `push`) can seed the next chain's `first_unit` correctly instead of losing its
    /// unit position.
    last_unit: Option<DVec3>,
}

impl ChainBuilder {
    fn new(target: TileCoordinate) -> Self {
        Self {
            target,
            active: false,
            points: Vec::new(),
            first_unit: None,
            last_unit: None,
        }
    }

    /// Flushes the in-progress chain (if it has at least 2 points) into `chains`, converting its
    /// world-space points to offsets relative to the target tile's center.
    fn flush(&mut self, chains: &mut Vec<RoadChain>, shape: TerrainShape, spherical: bool) {
        if self.points.len() >= 2 {
            let tile_count = 2f64.powi(self.target.lod as i32);
            let center_uv = (self.target.xy.as_dvec2() + 0.5) / tile_count;
            let center = Coordinate::new(self.target.face, center_uv).local_position(shape, 0.0);

            let mut cumulative = Vec::with_capacity(self.points.len());
            let mut waypoints = Vec::with_capacity(self.points.len());
            let mut distance = 0.0f32;
            for (i, point) in self.points.iter().enumerate() {
                if i > 0 {
                    distance += (*point - self.points[i - 1]).length() as f32;
                }
                cumulative.push(distance);
                waypoints.push((*point - center).as_vec3());
            }

            let normal = if spherical {
                let unit = self.first_unit.unwrap_or(DVec3::Y);
                (shape.scale() * unit).normalize().as_vec3()
            } else {
                Vec3::Y
            };

            chains.push(RoadChain {
                waypoints,
                cumulative,
                normal,
            });
        }

        self.points.clear();
        self.first_unit = None;
    }

    /// Adds a sample point. Flushes the chain in progress first if the sample isn't in the
    /// target tile, or if the chain is already full - in the full case the last point is carried
    /// over as the new chain's first point so adjacent chains still meet, keeping the rendered
    /// road visually continuous even though any single automobile only drives one chain of it.
    fn push(
        &mut self,
        located: &Located,
        chains: &mut Vec<RoadChain>,
        shape: TerrainShape,
        spherical: bool,
    ) {
        let in_target = located.tile == self.target;
        let full = self.points.len() >= MAX_WAYPOINTS;

        if !in_target {
            self.flush(chains, shape, spherical);
            self.active = false;
            self.last_unit = None;
            return;
        }

        if !self.active || full {
            let carry = (full && self.active)
                .then(|| self.points.last().copied())
                .flatten();
            let carry_unit = (full && self.active).then(|| self.last_unit).flatten();
            self.flush(chains, shape, spherical);
            self.active = true;
            if let Some(point) = carry {
                self.points.push(point);
                self.first_unit = carry_unit;
            }
        }

        if self.points.is_empty() {
            self.first_unit = Some(located.unit);
        }
        self.points.push(located.world);
        self.last_unit = Some(located.unit);
    }
}

/// Builds the road chains for a single active tile, on demand - the fine-grained,
/// elevation-aware counterpart to `roads::index::tile_road_occupancy`. Only resamples the roads
/// `candidates` (indices into `network.0`) names, which `roads::index::RoadCoarseIndex` has
/// already narrowed down to roads near `tile`'s coarse ancestor, so the expensive
/// per-tile-activation work stays proportional to how many roads are actually near the camera,
/// not the whole planet.
#[allow(clippy::too_many_arguments)]
pub fn chains_for_tile(
    tile: TileCoordinate,
    network: &roads::descriptor::RoadNetwork,
    candidates: &[u32],
    shape: TerrainShape,
    height_scale: f32,
    height_image: &Image,
    height_attachment: &Attachment,
) -> Vec<RoadChain> {
    let spherical = shape_is_spherical(shape);
    let tile_count = 2f64.powi(tile.lod as i32);

    let mut chains = Vec::new();

    for &road_index in candidates {
        let Some(road) = network.0.get(road_index as usize) else {
            continue;
        };
        if road.points.len() < 2 {
            continue;
        }

        let mut builder = ChainBuilder::new(tile);

        let (mut lon0, mut lat0) = road.points[0];
        let located = locate(
            lon0,
            lat0,
            shape,
            spherical,
            height_image,
            height_attachment,
            height_scale,
            tile,
            tile_count,
        );
        builder.push(&located, &mut chains, shape, spherical);

        for &(lon1, lat1) in &road.points[1..] {
            let located0 = locate(
                lon0,
                lat0,
                shape,
                spherical,
                height_image,
                height_attachment,
                height_scale,
                tile,
                tile_count,
            );
            let located1 = locate(
                lon1,
                lat1,
                shape,
                spherical,
                height_image,
                height_attachment,
                height_scale,
                tile,
                tile_count,
            );
            let segment_length = (located1.world - located0.world).length();
            let steps = ((segment_length / ROAD_SAMPLE_STEP_M).ceil() as u32).max(1);

            for step in 1..=steps {
                let t = step as f64 / steps as f64;
                let lon = lon0 + (lon1 - lon0) * t;
                let lat = lat0 + (lat1 - lat0) * t;
                let located = locate(
                    lon,
                    lat,
                    shape,
                    spherical,
                    height_image,
                    height_attachment,
                    height_scale,
                    tile,
                    tile_count,
                );
                builder.push(&located, &mut chains, shape, spherical);
            }

            lon0 = lon1;
            lat0 = lat1;
        }

        builder.flush(&mut chains, shape, spherical);
    }

    chains
}
