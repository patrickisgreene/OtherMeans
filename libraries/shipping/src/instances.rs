use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::sync_component::SyncComponent,
};
use big_space::prelude::Grids;
use shipping_lanes::descriptor::ShippingLaneNetwork;
use shipping_lanes::index::{ShippingLaneCoarseIndex, build_coarse_index, coarse_ancestor};
use terrain::math::Coordinate;
use terrain::prelude::{TileAtlas, TileCoordinate};

use crate::network::{MAX_WAYPOINTS, ShippingChain, chains_for_tile};

/// Target spacing (metres) between ships along a chain - lower means denser traffic.
const SHIP_SPACING_M: f32 = 20000.0;
/// Upper bound on ships spawned per chain, regardless of how long it is - keeps a single
/// unusually long in-tile chain (e.g. a highway running straight through a tile) from spawning
/// an excessive number of instances.
const MAX_SHIPS_PER_CHAIN: usize = 6;
/// Hard cap on ships spawned for a single tile, regardless of how many road chains it
/// contains. A shallow terrain config (few LODs) can make a single "highest LOD" tile hundreds of
/// kilometres across, potentially covering thousands of chains - without this, such a tile could
/// spawn tens of thousands of instances at once.
const MAX_SHIPS_PER_TILE: usize = 100;
/// Chains shorter than this aren't worth putting a ship on - the loop would be barely visible.
const MIN_CHAIN_LENGTH_M: f32 = 15.0;

const SHIP_SPEED_MIN: f32 = 1000.0;
const SHIP_SPEED_MAX: f32 = 2000.0;
// Longer, narrower and lower than a truck's near-cubic box - a container ship's silhouette is
// dominated by hull length, not width or height.
const SHIP_LENGTH: f32 = 1400.0;
const SHIP_WIDTH: f32 = 500.0;
const SHIP_HEIGHT: f32 = 500.0;

const SHIP_COLORS: [[f32; 3]; 5] = [
    [0.78, 0.10, 0.08],
    [0.85, 0.85, 0.88],
    [0.12, 0.12, 0.14],
    [0.10, 0.25, 0.55],
    [0.85, 0.65, 0.05],
];

/// Per-instance data for a single active tile, uploaded to the GPU as a vertex buffer. Unlike
/// `buildings::instances::InstanceData`, this is written once at spawn time but never describes
/// a fixed position - the vertex shader (`shaders/shipping.wgsl`) walks `waypoints` as a function
/// of a per-frame time uniform to animate the ship along its chain, so no per-frame CPU work
/// is needed to move it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    /// xyz = waypoint position (offset from the owning entity's tile-center position), w =
    /// cumulative distance (metres) from `waypoints[0]` up to and including this waypoint.
    /// Slots beyond `path_params.x` (the real waypoint count) repeat the last real waypoint so
    /// the shader's fixed-length loop is always safe to run to the end.
    pub waypoints: [[f32; 4]; MAX_WAYPOINTS],
    /// x = waypoint count, y = speed (m/s), z = phase offset (metres travelled at t=0), w =
    /// travel direction (+1.0 = forward along `waypoints`, -1.0 = reverse - see
    /// `build_instance`'s alternating lane assignment).
    pub path_params: [f32; 4],
    /// xyz = surface normal (constant per chain, see `RoadChain::normal`), w = unused padding.
    pub normal: [f32; 4],
    /// rgb = color, a = footprint width (metres, across the direction of travel).
    pub color_and_width: [f32; 4],
    /// x = length (metres, along the direction of travel), y = height (metres). z, w unused.
    pub dimensions: [f32; 4],
}

/// Identifies which terrain tile a ship batch entity belongs to.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShippingTile(pub TileCoordinate);

/// Per-instance ship data for a single active tile. Written once at tile spawn and never
/// mutated afterward (all motion happens in the vertex shader), so - exactly like
/// `buildings::instances::BuildingInstances` - it's extracted into the render world via a
/// hand-written `Changed<>`-gated system instead of `ExtractComponentPlugin`.
#[derive(Component, Clone)]
pub struct ShippingInstances(pub Vec<InstanceData>);

impl SyncComponent for ShippingInstances {
    type Target = ShippingInstances;
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

/// Deterministic pseudo-random unit float in `[0, 1)` for a given ship, stable across tile
/// respawns since it's derived purely from the tile/chain/ship indices (no external RNG
/// state) - mirrors `buildings::instances::jitter_uv`'s approach.
fn jitter_unit(
    coordinate: TileCoordinate,
    chain_index: usize,
    ship_index: usize,
    salt: u64,
) -> f32 {
    let seed = mix(
        mix(coordinate.face as u64, coordinate.xy.x as u64),
        mix(
            mix(coordinate.xy.y as u64, chain_index as u64),
            mix(ship_index as u64, salt),
        ),
    );
    (hash_u64(seed) as f64 / u64::MAX as f64) as f32
}

fn build_instance(
    chain: &ShippingChain,
    coordinate: TileCoordinate,
    chain_index: usize,
    ship_index: usize,
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
    let speed = SHIP_SPEED_MIN
        + jitter_unit(coordinate, chain_index, ship_index, 0) * (SHIP_SPEED_MAX - SHIP_SPEED_MIN);
    let phase = jitter_unit(coordinate, chain_index, ship_index, 1) * total_length;
    let color = SHIP_COLORS[(jitter_unit(coordinate, chain_index, ship_index, 2)
        * SHIP_COLORS.len() as f32)
        .min(SHIP_COLORS.len() as f32 - 1.0) as usize];
    // Alternates by `slot`, the ship's index across the *whole tile* rather than just its own
    // chain - most chains only spawn a single ship (`generate_tile_instances`'s per-chain
    // `count` is usually clamped down to 1), so alternating by `ship_index` alone left nearly
    // every ship at index 0 and therefore always forward. `slot` keeps the 50/50 split even
    // then; chains that do spawn multiple ships still get an even mix among themselves since
    // their ships occupy consecutive slots. One lane per direction (see the lateral lane
    // offset in `shaders/shipping.wgsl`).
    let direction = if slot % 2 == 0 { 1.0 } else { -1.0 };

    InstanceData {
        waypoints,
        path_params: [count as f32, speed, phase, direction],
        normal: [chain.normal.x, chain.normal.y, chain.normal.z, 0.0],
        color_and_width: [color[0], color[1], color[2], SHIP_WIDTH],
        dimensions: [SHIP_LENGTH, SHIP_HEIGHT, 0.0, 0.0],
    }
}

/// Generates the ship instances for every chain the road index has registered for
/// `coordinate`, spawning a density of ships proportional to each chain's length - capped at
/// `MAX_SHIPS_PER_TILE` in total. A "highest LOD" tile's real-world size depends entirely on
/// the terrain config's `lod_count` (a shallow config can make it hundreds of kilometres across,
/// easily covering thousands of chains), so the per-chain cap alone isn't enough to bound cost -
/// without a tile-wide budget a single very road-dense tile could spawn tens of thousands of
/// instances and blow well past the GPU's memory.
fn generate_tile_instances(
    coordinate: TileCoordinate,
    chains: &[ShippingChain],
) -> Vec<InstanceData> {
    let total_length: f32 = chains.iter().map(ShippingChain::total_length).sum();
    if total_length < MIN_CHAIN_LENGTH_M {
        return Vec::new();
    }

    let desired_total = ((total_length / SHIP_SPACING_M).round() as usize).max(1);
    let budget = desired_total.min(MAX_SHIPS_PER_TILE);

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

        // Proportional share of the tile's budget, so longer chains get more ships, capped
        // per-chain so one exceptionally long chain can't visually hog the whole tile's budget.
        let share = ((length / total_length) * budget as f32).round() as usize;
        let count = share.clamp(1, MAX_SHIPS_PER_CHAIN).min(budget - assigned);

        for ship_index in 0..count {
            let slot = instances.len();
            instances.push(build_instance(
                chain,
                coordinate,
                chain_index,
                ship_index,
                slot,
            ));
        }
        assigned += count;
    }

    instances
}

/// Keeps one ship-batch entity alive per terrain tile that is currently loaded at the highest
/// LOD and has at least one road chain, spawning/despawning entities as the active tile set
/// changes - mirrors `buildings::instances::update_building_batches`. The whole-globe
/// `RoadTileIndex` is built exactly once, the first time a `TileAtlas` and the `RoadNetwork`
/// asset are both available (tile boundaries at the fixed highest LOD never change afterward, so
/// there's nothing to rebuild).
pub fn update_shipping_batches(
    mut commands: Commands,
    mut known: Local<HashMap<TileCoordinate, Option<Entity>>>,
    mut network_handle: Local<Option<Handle<ShippingLaneNetwork>>>,
    mut coarse_index: Local<Option<ShippingLaneCoarseIndex>>,
    asset_server: Res<AssetServer>,
    networks: Res<Assets<ShippingLaneNetwork>>,
    grids: Grids,
    terrain_query: Query<(Entity, &TileAtlas)>,
) {
    let network_handle = network_handle
        .get_or_insert_with(|| asset_server.load("earth/shipping.ron"))
        .clone();
    let Some(network) = networks.get(&network_handle) else {
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

            let chains = chains_for_tile(coordinate, network, candidates, tile_atlas.shape);
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
                    ShippingTile(coordinate),
                    ShippingInstances(instances),
                    cell,
                    Transform::from_translation(translation),
                    ChildOf(root),
                ))
                .id();

            known.insert(coordinate, Some(entity));
        }
    }
}
