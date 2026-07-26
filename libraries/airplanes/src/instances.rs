use bevy::{
    image::Image,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::sync_component::SyncComponent,
};
use big_space::prelude::Grids;
use cities::descriptor::CitiesDatabase;
use terrain::math::Coordinate;
use terrain::prelude::{AttachmentLabel, TileAtlas, TileCoordinate};

use buildings::tile_height::height_tile_path;

use crate::index::{AirplaneRouteCoarseIndex, build_coarse_index, coarse_ancestor};
use crate::network::{AirplaneChain, MAX_WAYPOINTS, chains_for_tile};
use crate::routes::{AirplaneRouteNetwork, build_route_network};

/// Target spacing (metres) between planes along a chain - air routes are far sparser than road or
/// shipping traffic, so this dwarfs `automobiles`'/`shipping`'s equivalent constants.
const AIRPLANE_SPACING_M: f32 = 400_000.0;
/// Upper bound on planes spawned per chain, regardless of how long it is.
const MAX_AIRPLANES_PER_CHAIN: usize = 7;
/// Hard cap on planes spawned for a single tile, regardless of how many chains it contains.
const MAX_AIRPLANES_PER_TILE: usize = 40;
/// Chains shorter than this aren't worth putting a plane on.
const MIN_CHAIN_LENGTH_M: f32 = 500.0;

const AIRPLANE_SPEED_MIN: f32 = 800.0;
const AIRPLANE_SPEED_MAX: f32 = 1200.0;
// A slim fuselage silhouette - long and narrow along the direction of travel, unlike a ship's
// broad hull or a truck's near-cubic box.
const AIRPLANE_LENGTH: f32 = 3000.0;
const AIRPLANE_WIDTH: f32 = 5000.0;
const AIRPLANE_HEIGHT: f32 = 2000.0;

const AIRPLANE_COLORS: [[f32; 3]; 3] = [[0.92, 0.92, 0.95], [0.85, 0.15, 0.12], [0.10, 0.35, 0.75]];

/// Per-instance data for a single active tile, uploaded to the GPU as a vertex buffer. Copy of
/// `shipping::instances::InstanceData` - the vertex shader (`shaders/airplanes.wgsl`) walks
/// `waypoints` as a function of a per-frame time uniform to animate the plane along its chain.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub waypoints: [[f32; 4]; MAX_WAYPOINTS],
    pub path_params: [f32; 4],
    pub normal: [f32; 4],
    pub color_and_width: [f32; 4],
    pub dimensions: [f32; 4],
}

/// Identifies which terrain tile a plane batch entity belongs to.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AirplaneTile(pub TileCoordinate);

/// Per-instance plane data for a single active tile. Written once at tile spawn and never mutated
/// afterward (all motion happens in the vertex shader) - copy of
/// `shipping::instances::ShippingInstances`.
#[derive(Component, Clone)]
pub struct AirplaneInstances(pub Vec<InstanceData>);

impl SyncComponent for AirplaneInstances {
    type Target = AirplaneInstances;
}

fn hash_u64(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

fn mix(a: u64, b: u64) -> u64 {
    hash_u64(a ^ hash_u64(b))
}

/// Deterministic pseudo-random unit float in `[0, 1)` for a given plane, stable across tile
/// respawns. Copy of `shipping::instances::jitter_unit`.
fn jitter_unit(
    coordinate: TileCoordinate,
    chain_index: usize,
    airplane_index: usize,
    salt: u64,
) -> f32 {
    let seed = mix(
        mix(coordinate.face as u64, coordinate.xy.x as u64),
        mix(
            mix(coordinate.xy.y as u64, chain_index as u64),
            mix(airplane_index as u64, salt),
        ),
    );
    (hash_u64(seed) as f64 / u64::MAX as f64) as f32
}

fn build_instance(
    chain: &AirplaneChain,
    coordinate: TileCoordinate,
    chain_index: usize,
    airplane_index: usize,
    slot: usize,
) -> InstanceData {
    let count = chain.waypoints.len().min(MAX_WAYPOINTS);

    let mut waypoints = [[0.0f32; 4]; MAX_WAYPOINTS];
    for i in 0..MAX_WAYPOINTS {
        let source = i.min(count - 1);
        let position = chain.waypoints[source];
        waypoints[i] = [position.x, position.y, position.z, chain.cumulative[source]];
    }

    let total_length = chain.total_length().max(0.0001);
    let speed = AIRPLANE_SPEED_MIN
        + jitter_unit(coordinate, chain_index, airplane_index, 0)
            * (AIRPLANE_SPEED_MAX - AIRPLANE_SPEED_MIN);
    let phase = jitter_unit(coordinate, chain_index, airplane_index, 1) * total_length;
    let color = AIRPLANE_COLORS[(jitter_unit(coordinate, chain_index, airplane_index, 2)
        * AIRPLANE_COLORS.len() as f32)
        .min(AIRPLANE_COLORS.len() as f32 - 1.0) as usize];
    let direction = if slot % 2 == 0 { 1.0 } else { -1.0 };

    InstanceData {
        waypoints,
        path_params: [count as f32, speed, phase, direction],
        normal: [chain.normal.x, chain.normal.y, chain.normal.z, 0.0],
        color_and_width: [color[0], color[1], color[2], AIRPLANE_WIDTH],
        dimensions: [AIRPLANE_LENGTH, AIRPLANE_HEIGHT, 0.0, 0.0],
    }
}

/// Generates the plane instances for every chain the route index has registered for
/// `coordinate`. Copy of `shipping::instances::generate_tile_instances`, retuned for sparser air
/// traffic.
fn generate_tile_instances(
    coordinate: TileCoordinate,
    chains: &[AirplaneChain],
) -> Vec<InstanceData> {
    let total_length: f32 = chains.iter().map(AirplaneChain::total_length).sum();
    if total_length < MIN_CHAIN_LENGTH_M {
        return Vec::new();
    }

    let desired_total = ((total_length / AIRPLANE_SPACING_M).round() as usize).max(1);
    let budget = desired_total.min(MAX_AIRPLANES_PER_TILE);

    let mut instances = Vec::with_capacity(budget);
    let mut assigned = 0usize;

    for (chain_index, chain) in chains.iter().enumerate() {
        if assigned >= budget {
            break;
        }

        let length = chain.total_length();
        if length < MIN_CHAIN_LENGTH_M {
            continue;
        }

        let share = ((length / total_length) * budget as f32).round() as usize;
        let count = share
            .clamp(1, MAX_AIRPLANES_PER_CHAIN)
            .min(budget - assigned);

        for airplane_index in 0..count {
            let slot = instances.len();
            instances.push(build_instance(
                chain,
                coordinate,
                chain_index,
                airplane_index,
                slot,
            ));
        }
        assigned += count;
    }

    instances
}

/// Keeps one plane-batch entity alive per terrain tile that is currently loaded at the highest
/// LOD and has at least one route chain, spawning/despawning entities as the active tile set
/// changes. Copy of `shipping::instances::update_shipping_batches`, except the route network
/// itself is built once at runtime from `CitiesDatabase` (via `routes::build_route_network`)
/// instead of loaded from a preprocessed asset - there is no flight-path shapefile to load.
pub fn update_airplane_batches(
    mut commands: Commands,
    mut known: Local<HashMap<TileCoordinate, Option<Entity>>>,
    mut cities_handle: Local<Option<Handle<CitiesDatabase>>>,
    mut route_network: Local<Option<AirplaneRouteNetwork>>,
    mut coarse_index: Local<Option<AirplaneRouteCoarseIndex>>,
    mut height_tile_handles: Local<HashMap<TileCoordinate, Handle<Image>>>,
    asset_server: Res<AssetServer>,
    cities_databases: Res<Assets<CitiesDatabase>>,
    images: Res<Assets<Image>>,
    grids: Grids,
    terrain_query: Query<(Entity, &TileAtlas)>,
) {
    let cities_handle = cities_handle
        .get_or_insert_with(|| asset_server.load("earth/cities.ron"))
        .clone();

    if route_network.is_none() {
        let Some(cities) = cities_databases.get(&cities_handle) else {
            return;
        };
        *route_network = Some(build_route_network(cities));
    }
    let Some(network) = route_network.as_ref() else {
        return;
    };

    if coarse_index.is_none() {
        let Some((_, tile_atlas)) = terrain_query.iter().next() else {
            return;
        };
        let target_lod = tile_atlas.lod_count.saturating_sub(1);
        let built = build_coarse_index(network, tile_atlas.shape, target_lod);
        *coarse_index = Some(built);
    }
    let Some(coarse_index) = coarse_index.as_ref() else {
        return;
    };

    for (terrain_entity, tile_atlas) in &terrain_query {
        let Some(root) = grids.parent_grid_entity(terrain_entity) else {
            continue;
        };
        let grid = grids.get(root);
        let Some(height_attachment) = tile_atlas.attachments.get(&AttachmentLabel::Height) else {
            continue;
        };

        let active: HashSet<TileCoordinate> = tile_atlas.active_tiles_at_highest_lod().collect();

        known.retain(|coordinate, entity| {
            if active.contains(coordinate) {
                true
            } else {
                if let Some(entity) = entity {
                    commands.entity(*entity).despawn();
                }
                false
            }
        });
        height_tile_handles.retain(|coordinate, _| active.contains(coordinate));

        let new_tiles: Vec<TileCoordinate> = active
            .iter()
            .filter(|c| !known.contains_key(*c))
            .copied()
            .collect();

        for coordinate in new_tiles {
            let ancestor = coarse_ancestor(coordinate, coarse_index.coarse_lod);
            let Some(candidates) = coarse_index.buckets.get(&ancestor) else {
                known.insert(coordinate, None);
                continue;
            };

            // Load (or keep waiting on) this tile's own height image - the exact same per-tile
            // R32F file the terrain mesh itself displaces by - before placing planes above it.
            // Left out of `known` (retried next frame) until it's ready.
            let handle = height_tile_handles
                .entry(coordinate)
                .or_insert_with(|| {
                    asset_server.load(height_tile_path(coordinate, height_attachment))
                })
                .clone();
            let Some(height_image) = images.get(&handle) else {
                continue;
            };

            let chains = chains_for_tile(
                coordinate,
                network,
                candidates,
                tile_atlas.shape,
                tile_atlas.height_scale,
                height_image,
                height_attachment,
            );
            let instances = generate_tile_instances(coordinate, &chains);
            if instances.is_empty() {
                known.insert(coordinate, None);
                continue;
            }

            let tile_count = 2f64.powi(coordinate.lod as i32);
            let center_uv = (coordinate.xy.as_dvec2() + 0.5) / tile_count;
            let center_world =
                Coordinate::new(coordinate.face, center_uv).local_position(tile_atlas.shape, 0.0);
            let (cell, translation) = grid.translation_to_grid(center_world);

            let entity = commands
                .spawn((
                    AirplaneTile(coordinate),
                    AirplaneInstances(instances),
                    cell,
                    Transform::from_translation(translation),
                    ChildOf(root),
                ))
                .id();

            known.insert(coordinate, Some(entity));
        }
    }
}
