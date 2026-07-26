use bevy::{image::Image, math::DVec3, prelude::*};
use terrain::data::attachment::Attachment;
use terrain::math::Coordinate;
use terrain::prelude::{TerrainShape, TileCoordinate};

use buildings::tile_height::{sample_height_tile, tile_local_uv};

use crate::index::{shape_is_spherical, tile_xy};
use crate::routes::AirplaneRouteNetwork;
use workspace::lat_lon_to_unit_position;

/// Real-world spacing (metres) at which a route's raw (already ~50km-spaced, see
/// `routes::GREAT_CIRCLE_SAMPLE_STEP_M`) points are resampled when building the fine, tile-local
/// chains for one active tile. Coarser than roads'/ships' equivalent constant since a route's raw
/// points are already close together relative to a highest-LOD tile.
const ROUTE_SAMPLE_STEP_M: f64 = 2_000.0;

/// Cruising altitude (metres above the *real, heightmap-displaced ground* - see `locate`'s use of
/// `buildings::tile_height`, not the idealized flat ellipsoid) a route eases up to away from its
/// endpoints. Earlier revisions computed altitude above the flat ellipsoid baseline instead of
/// real ground height, which put planes visually underground almost everywhere on land (the
/// opaque terrain mesh, displaced upward by its own heightmap, occluded them) - confirmed via a
/// GPU-side draw_indexed call that executed correctly with real instance data yet never produced
/// visible pixels. Layering this on top of real elevation fixes that.
const CRUISE_ALTITUDE_M: f64 = 1_500.0;

/// Distance (metres) from each endpoint over which altitude eases between real ground level and
/// `CRUISE_ALTITUDE_M`, giving flights a climb/cruise/descend profile instead of popping straight
/// to cruise. Capped per-route (see `altitude_for_progress`) so a short hub-to-spoke hop still
/// gets a full climb-then-descend arc rather than holding at cruise for zero distance.
const CLIMB_DISTANCE_M: f64 = 20_000.0;

/// Maximum number of waypoints stored per in-tile route chain. Bounds the size of the
/// per-instance waypoint array the vertex shader walks (see `render::pipeline`).
pub const MAX_WAYPOINTS: usize = 8;

/// A single contiguous, tile-local sequence of points a plane can loop along. Always wholly
/// contained within one terrain tile. Copy of `shipping::network::ShippingChain`.
#[derive(Clone)]
pub struct AirplaneChain {
    /// Waypoint positions, relative to the owning tile's center.
    pub waypoints: Vec<Vec3>,
    /// Cumulative real-world distance (metres) from `waypoints[0]` to each waypoint; same length
    /// as `waypoints`, `cumulative[0] == 0.0`.
    pub cumulative: Vec<f32>,
    /// "Up" normal used for every plane on this chain - at cruise altitude the surface normal is
    /// still a fine stand-in for "up", exactly as it is at ground level for ships/automobiles.
    pub normal: Vec3,
}

impl AirplaneChain {
    pub fn total_length(&self) -> f32 {
        self.cumulative.last().copied().unwrap_or(0.0)
    }
}

/// Eases altitude between ground level and `CRUISE_ALTITUDE_M` over `CLIMB_DISTANCE_M` at each
/// end of the route, capped to half the route's `total_length` so short hops still climb and
/// descend fully rather than holding at cruise for zero distance.
fn altitude_for_progress(distance_from_start: f64, total_length: f64) -> f64 {
    let climb = CLIMB_DISTANCE_M.min(total_length * 0.5).max(1.0);
    let from_start = (distance_from_start / climb).clamp(0.0, 1.0);
    let from_end = ((total_length - distance_from_start) / climb).clamp(0.0, 1.0);
    CRUISE_ALTITUDE_M * from_start.min(from_end)
}

/// A (lon, lat) point resolved to `target_lod`'s tile grid, plus its real world-space position
/// (lifted above the *real, heightmap-displaced ground* by this point's eased cruise altitude -
/// see `buildings::tile_height`) and its unit cube-sphere position (used to derive a surface
/// normal). Copy of `automobiles::network::Located`, plus the cruise-altitude easing on top of
/// real elevation that `automobiles` (which stays right at ground level) doesn't need.
struct Located {
    tile: TileCoordinate,
    world: DVec3,
    unit: DVec3,
}

#[allow(clippy::too_many_arguments)]
fn locate(
    lon_deg: f64,
    lat_deg: f64,
    distance_from_start: f64,
    total_length: f64,
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

    // Sampled against `target`'s own height tile even for candidate points that turn out to fall
    // outside it, exactly like `automobiles::network::locate` - those points get discarded by
    // `ChainBuilder::push`'s `in_target` check regardless.
    let local_uv = tile_local_uv(coordinate.uv, target);
    let ground_elevation = sample_height_tile(height_image, height_attachment, local_uv) * height_scale;
    let altitude = ground_elevation as f64 + altitude_for_progress(distance_from_start, total_length);
    let world = shape.position_unit_to_local(unit, altitude);

    let xy = tile_xy(coordinate, tile_count);

    Located {
        tile: TileCoordinate::new(coordinate.face, target.lod, xy),
        world,
        unit,
    }
}

/// Accumulates consecutive samples that fall inside `target` into `AirplaneChain`s, discarding
/// any run of samples that falls outside it. Copy of `shipping::network::ChainBuilder`.
struct ChainBuilder {
    target: TileCoordinate,
    active: bool,
    points: Vec<DVec3>,
    first_unit: Option<DVec3>,
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

    fn flush(&mut self, chains: &mut Vec<AirplaneChain>, shape: TerrainShape, spherical: bool) {
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

            chains.push(AirplaneChain {
                waypoints,
                cumulative,
                normal,
            });
        }

        self.points.clear();
        self.first_unit = None;
    }

    fn push(
        &mut self,
        located: &Located,
        chains: &mut Vec<AirplaneChain>,
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

/// Builds the flight chains for a single active tile, on demand - the fine-grained,
/// altitude-aware counterpart to `index::AirplaneRouteCoarseIndex`. Only resamples the routes
/// `candidates` (indices into `network.0`) names, which the coarse index has already narrowed
/// down to routes near `tile`'s coarse ancestor.
#[allow(clippy::too_many_arguments)]
pub fn chains_for_tile(
    tile: TileCoordinate,
    network: &AirplaneRouteNetwork,
    candidates: &[u32],
    shape: TerrainShape,
    height_scale: f32,
    height_image: &Image,
    height_attachment: &Attachment,
) -> Vec<AirplaneChain> {
    let spherical = shape_is_spherical(shape);
    let tile_count = 2f64.powi(tile.lod as i32);

    let mut chains = Vec::new();

    for &route_index in candidates {
        let Some(route) = network.0.get(route_index as usize) else {
            continue;
        };
        if route.points.len() < 2 {
            continue;
        }

        // `total_length` here is the *original, unchunked* route's full length - needed so
        // altitude eases relative to its true endpoints, not this chunk's own ends (see
        // `routes::AirplaneRouteDescriptor`'s doc comment). `cumulative_at` returns each point's
        // distance from that same true start: points are evenly spaced by angle within a chunk
        // (see `routes::sample_great_circle`), so it's a simple interpolation between this
        // chunk's own start/end distances.
        let total_length = route.route_total_length_m;
        let last_index = route.points.len() - 1;
        let chunk_length = route.end_distance_m - route.start_distance_m;
        let cumulative_at = |index: usize| {
            route.start_distance_m + (index as f64 / last_index as f64) * chunk_length
        };

        let mut builder = ChainBuilder::new(tile);

        let (mut lon0, mut lat0) = route.points[0];
        let located = locate(
            lon0,
            lat0,
            route.start_distance_m,
            total_length,
            shape,
            spherical,
            height_image,
            height_attachment,
            height_scale,
            tile,
            tile_count,
        );
        builder.push(&located, &mut chains, shape, spherical);

        for (segment_index, &(lon1, lat1)) in route.points[1..].iter().enumerate() {
            let distance0 = cumulative_at(segment_index);
            let distance1 = cumulative_at(segment_index + 1);

            let located0 = locate(
                lon0,
                lat0,
                distance0,
                total_length,
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
                distance1,
                total_length,
                shape,
                spherical,
                height_image,
                height_attachment,
                height_scale,
                tile,
                tile_count,
            );
            let segment_length = (located1.world - located0.world).length();
            let steps = ((segment_length / ROUTE_SAMPLE_STEP_M).ceil() as u32).max(1);

            for step in 1..=steps {
                let t = step as f64 / steps as f64;
                let lon = lon0 + (lon1 - lon0) * t;
                let lat = lat0 + (lat1 - lat0) * t;
                let distance = distance0 + (distance1 - distance0) * t;
                let located = locate(
                    lon,
                    lat,
                    distance,
                    total_length,
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
