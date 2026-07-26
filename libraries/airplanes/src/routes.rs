use bevy::{math::DVec3, prelude::*};
use cities::descriptor::{CitiesDatabase, CityDescriptor};
use workspace::lat_lon_to_unit_position;

/// Real-world spacing (metres) at which a great-circle route is sampled into raw (lon, lat)
/// points before chunking. Coarser than `network::ROUTE_SAMPLE_STEP_M` (which further resamples
/// between these points when building tile-local chains) - this only needs to be fine enough
/// that linear interpolation between consecutive points stays close to the true geodesic.
const GREAT_CIRCLE_SAMPLE_STEP_M: f64 = 50_000.0;

/// Raw points per chunked [`AirplaneRouteDescriptor`] (see `chunk_route`). A hub-to-hub route can
/// span thousands of kilometres and hundreds of raw points; `network::chains_for_tile` has to
/// walk a whole candidate descriptor's points to find the ones near one terrain tile, so leaving
/// a route as a single multi-thousand-point descriptor makes every tile lookup pay for the whole
/// route's length, however far from that tile most of it is. Real-world shapefiles (which
/// `roads`/`shipping-lanes` load) happen to already be chopped into short "parts" for the same
/// reason - this splits generated routes the same way, at generation time, so per-tile lookups
/// stay proportional to a single chunk's length instead of a whole route's.
const ROUTE_CHUNK_POINTS: usize = 20;

/// How many cities become hub airports, ranked by (`scale_rank` ascending, population
/// descending) - i.e. the `HUB_COUNT` most important cities in the whole database. Hubs connect
/// to every other hub (`HUB_COUNT^2` routes), so this must stay small; every other qualifying
/// city instead connects only to its nearest hub.
const HUB_COUNT: usize = 40;

/// Cities smaller than this are assumed to have no scheduled air service and are skipped when
/// generating spoke routes - keeps the network from connecting every small town.
const MIN_SPOKE_POPULATION: u32 = 50_000;

/// A single flight route chunk, as a sequence of (longitude, latitude) points in degrees sampled
/// along the great-circle path between two cities, in travel order. One hub-to-hub or
/// hub-to-spoke connection is generally split into several of these by `chunk_route` - each
/// chunk is independently sized for `network::chains_for_tile`'s per-tile candidate walk, but
/// `start_distance_m`/`end_distance_m`/`route_total_length_m` still describe this chunk's
/// position within the *original, unchunked* route, so altitude easing (which needs distance
/// from the true endpoints, not from this chunk's own ends) stays correct across chunk
/// boundaries. Mirrors `shipping_lanes::descriptor::ShippingLaneDescriptor`'s shape plus that
/// bookkeeping.
#[derive(Debug, PartialEq, Clone)]
pub struct AirplaneRouteDescriptor {
    pub points: Vec<(f64, f64)>,
    /// This chunk's first point's distance (metres) from the original route's start.
    pub start_distance_m: f64,
    /// This chunk's last point's distance (metres) from the original route's start.
    pub end_distance_m: f64,
    /// The *original, unchunked* route's total length (metres) - both endpoints' distance from
    /// each other, used by `network::altitude_for_progress` to ease altitude near either end.
    pub route_total_length_m: f64,
}

/// The whole hub-and-spoke flight network, built once at runtime from a loaded
/// [`CitiesDatabase`]. Unlike `shipping_lanes::descriptor::ShippingLaneNetwork`, this isn't loaded
/// from disk - there is no real-world "flight paths" shapefile to preprocess - so it's a plain
/// resource rather than a Bevy `Asset`.
#[derive(Resource, Default)]
pub struct AirplaneRouteNetwork(pub Vec<AirplaneRouteDescriptor>);

/// Inverse of `workspace::lat_lon_to_unit_position`. Private copy of
/// `buildings::instances::lat_lon_degrees`, which isn't `pub`.
fn lat_lon_degrees(unit_position: DVec3) -> (f64, f64) {
    let lon = unit_position.z.atan2(-unit_position.x);
    let lat = unit_position.y.asin();
    (lat.to_degrees(), lon.to_degrees())
}

/// Great-circle distance in metres between two cities, used both to find each spoke's nearest hub
/// and to size each route's sample count.
fn great_circle_distance_m(a: &CityDescriptor, b: &CityDescriptor) -> f64 {
    let start = lat_lon_to_unit_position(a.lat, a.lon);
    let end = lat_lon_to_unit_position(b.lat, b.lon);
    start.angle_between(end) * workspace::EARTH_RADIUS
}

/// Splits `points` (evenly spaced by angle, `route_total_length_m` end to end) into
/// `AirplaneRouteDescriptor` chunks of at most `ROUTE_CHUNK_POINTS` points, each overlapping its
/// neighbour by one point so the polyline stays geometrically continuous across the split.
fn chunk_route(points: &[(f64, f64)], route_total_length_m: f64) -> Vec<AirplaneRouteDescriptor> {
    let step_count = points.len() - 1;
    if step_count == 0 {
        return Vec::new();
    }
    let step_length_m = route_total_length_m / step_count as f64;
    let stride = ROUTE_CHUNK_POINTS.saturating_sub(1).max(1);

    let mut descriptors = Vec::new();
    let mut start = 0usize;
    loop {
        let end = (start + stride).min(step_count);
        descriptors.push(AirplaneRouteDescriptor {
            points: points[start..=end].to_vec(),
            start_distance_m: start as f64 * step_length_m,
            end_distance_m: end as f64 * step_length_m,
            route_total_length_m,
        });
        if end == step_count {
            break;
        }
        start = end;
    }
    descriptors
}

/// Samples the great-circle path between two cities at roughly `GREAT_CIRCLE_SAMPLE_STEP_M`
/// spacing via spherical linear interpolation between their unit-sphere positions (this stays on
/// the sphere at every step, unlike interpolating lon/lat directly, so widely-separated endpoints
/// still produce a genuine geodesic instead of a straight line on the equirectangular
/// projection), then splits the result into `chunk_route`'s short pieces.
fn sample_great_circle(from: &CityDescriptor, to: &CityDescriptor) -> Vec<AirplaneRouteDescriptor> {
    let start = lat_lon_to_unit_position(from.lat, from.lon);
    let end = lat_lon_to_unit_position(to.lat, to.lon);

    let angle = start.angle_between(end);
    if angle < 1e-9 {
        return Vec::new();
    }

    let route_total_length_m = angle * workspace::EARTH_RADIUS;
    let steps = ((route_total_length_m / GREAT_CIRCLE_SAMPLE_STEP_M).ceil() as u32).max(1);
    let sin_angle = angle.sin();

    let points: Vec<(f64, f64)> = (0..=steps)
        .map(|step| {
            let t = step as f64 / steps as f64;
            let a = ((1.0 - t) * angle).sin() / sin_angle;
            let b = (t * angle).sin() / sin_angle;
            let (lat, lon) = lat_lon_degrees((start * a + end * b).normalize());
            (lon, lat)
        })
        .collect();

    chunk_route(&points, route_total_length_m)
}

/// Builds the hub-and-spoke flight network from the loaded cities database: the `HUB_COUNT`
/// highest-ranked cities become hubs connected to every other hub, and every other city with at
/// least `MIN_SPOKE_POPULATION` people connects to its nearest hub.
pub fn build_route_network(cities: &CitiesDatabase) -> AirplaneRouteNetwork {
    let mut ranked: Vec<&CityDescriptor> = cities.0.iter().collect();
    ranked.sort_by(|a, b| {
        a.scale_rank
            .cmp(&b.scale_rank)
            .then_with(|| b.population.cmp(&a.population))
    });
    let hubs: Vec<&CityDescriptor> = ranked.into_iter().take(HUB_COUNT).collect();

    if hubs.is_empty() {
        return AirplaneRouteNetwork::default();
    }

    let hub_ids: bevy::platform::collections::HashSet<u32> = hubs.iter().map(|c| c.id).collect();
    let mut routes = Vec::new();

    for (index, hub) in hubs.iter().enumerate() {
        for other in &hubs[index + 1..] {
            routes.extend(sample_great_circle(hub, other));
        }
    }

    for city in &cities.0 {
        if hub_ids.contains(&city.id) || city.population < MIN_SPOKE_POPULATION {
            continue;
        }

        let nearest_hub = hubs
            .iter()
            .min_by(|a, b| {
                great_circle_distance_m(city, a).total_cmp(&great_circle_distance_m(city, b))
            })
            .expect("hubs is non-empty");

        routes.extend(sample_great_circle(city, nearest_hub));
    }

    AirplaneRouteNetwork(routes)
}
