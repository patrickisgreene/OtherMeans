use bevy::{
    image::Image,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::sync_component::SyncComponent,
};
use big_space::prelude::Grids;
use terrain::math::Coordinate;
use terrain::prelude::{AttachmentLabel, TileAtlas, TileCoordinate};

use buildings::tile_height::height_tile_path;
use roads::descriptor::RoadNetwork;
use roads::index::{RoadCoarseIndex, build_coarse_index, coarse_ancestor};

use crate::network::{MAX_WAYPOINTS, RoadChain, chains_for_tile};

/// Target spacing (metres) between automobiles along a chain - lower means denser traffic.
const AUTOMOBILES_SPACING_M: f32 = 2500.0;
/// Upper bound on automobiles spawned per chain, regardless of how long it is - keeps a single
/// unusually long in-tile chain (e.g. a highway running straight through a tile) from spawning
/// an excessive number of instances.
const MAX_AUTOMOBILES_PER_CHAIN: usize = 6;
/// Hard cap on automobiles spawned for a single tile, regardless of how many road chains it
/// contains. A shallow terrain config (few LODs) can make a single "highest LOD" tile hundreds of
/// kilometres across, potentially covering thousands of chains - without this, such a tile could
/// spawn tens of thousands of instances at once.
const MAX_AUTOMOBILES_PER_TILE: usize = 150;
/// Chains shorter than this aren't worth putting a automobile on - the loop would be barely visible.
const MIN_CHAIN_LENGTH_M: f32 = 15.0;

const AUTOMOBILE_SPEED_MIN: f32 = 100.0;
const AUTOMOBILE_SPEED_MAX: f32 = 500.0;
const AUTOMOBILE_LENGTH: f32 = 1000.5;
const AUTOMOBILE_WIDTH: f32 = 250.9;
const AUTOMOBILE_HEIGHT: f32 = 400.5;

const AUTOMOBILE_COLORS: [[f32; 3]; 5] = [
    [0.78, 0.10, 0.08],
    [0.85, 0.85, 0.88],
    [0.12, 0.12, 0.14],
    [0.10, 0.25, 0.55],
    [0.85, 0.65, 0.05],
];

/// Per-instance data for a single active tile, uploaded to the GPU as a vertex buffer. Unlike
/// `buildings::instances::InstanceData`, this is written once at spawn time but never describes
/// a fixed position - the vertex shader (`shaders/automobiles.wgsl`) walks `waypoints` as a function
/// of a per-frame time uniform to animate the automobile along its chain, so no per-frame CPU work
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

/// Identifies which terrain tile a automobile batch entity belongs to.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AutomobilesTile(pub TileCoordinate);

/// Per-instance automobile data for a single active tile. Written once at tile spawn and never
/// mutated afterward (all motion happens in the vertex shader), so - exactly like
/// `buildings::instances::BuildingInstances` - it's extracted into the render world via a
/// hand-written `Changed<>`-gated system instead of `ExtractComponentPlugin`.
#[derive(Component, Clone)]
pub struct AutomobilesInstances(pub Vec<InstanceData>);

impl SyncComponent for AutomobilesInstances {
    type Target = AutomobilesInstances;
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

/// Deterministic pseudo-random unit float in `[0, 1)` for a given automobile, stable across tile
/// respawns since it's derived purely from the tile/chain/automobile indices (no external RNG
/// state) - mirrors `buildings::instances::jitter_uv`'s approach.
fn jitter_unit(
    coordinate: TileCoordinate,
    chain_index: usize,
    automobile_index: usize,
    salt: u64,
) -> f32 {
    let seed = mix(
        mix(coordinate.face as u64, coordinate.xy.x as u64),
        mix(
            mix(coordinate.xy.y as u64, chain_index as u64),
            mix(automobile_index as u64, salt),
        ),
    );
    (hash_u64(seed) as f64 / u64::MAX as f64) as f32
}

fn build_instance(
    chain: &RoadChain,
    coordinate: TileCoordinate,
    chain_index: usize,
    automobile_index: usize,
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
    let speed = AUTOMOBILE_SPEED_MIN
        + jitter_unit(coordinate, chain_index, automobile_index, 0)
            * (AUTOMOBILE_SPEED_MAX - AUTOMOBILE_SPEED_MIN);
    let phase = jitter_unit(coordinate, chain_index, automobile_index, 1) * total_length;
    let color = AUTOMOBILE_COLORS[(jitter_unit(coordinate, chain_index, automobile_index, 2)
        * AUTOMOBILE_COLORS.len() as f32)
        .min(AUTOMOBILE_COLORS.len() as f32 - 1.0) as usize];
    // Alternates by `slot`, the automobile's index across the *whole tile* rather than just its own
    // chain - most chains only spawn a single automobile (`generate_tile_instances`'s per-chain
    // `count` is usually clamped down to 1), so alternating by `automobile_index` alone left nearly
    // every automobile at index 0 and therefore always forward. `slot` keeps the 50/50 split even
    // then; chains that do spawn multiple automobiles still get an even mix among themselves since
    // their automobiles occupy consecutive slots. One lane per direction (see the lateral lane
    // offset in `shaders/automobiles.wgsl`).
    let direction = if slot % 2 == 0 { 1.0 } else { -1.0 };

    InstanceData {
        waypoints,
        path_params: [count as f32, speed, phase, direction],
        normal: [chain.normal.x, chain.normal.y, chain.normal.z, 0.0],
        color_and_width: [color[0], color[1], color[2], AUTOMOBILE_WIDTH],
        dimensions: [AUTOMOBILE_LENGTH, AUTOMOBILE_HEIGHT, 0.0, 0.0],
    }
}

/// Generates the automobile instances for every chain the road index has registered for
/// `coordinate`, spawning a density of automobiles proportional to each chain's length - capped at
/// `MAX_AUTOMOBILES_PER_TILE` in total. A "highest LOD" tile's real-world size depends entirely on
/// the terrain config's `lod_count` (a shallow config can make it hundreds of kilometres across,
/// easily covering thousands of chains), so the per-chain cap alone isn't enough to bound cost -
/// without a tile-wide budget a single very road-dense tile could spawn tens of thousands of
/// instances and blow well past the GPU's memory.
fn generate_tile_instances(coordinate: TileCoordinate, chains: &[RoadChain]) -> Vec<InstanceData> {
    let total_length: f32 = chains.iter().map(RoadChain::total_length).sum();
    if total_length < MIN_CHAIN_LENGTH_M {
        return Vec::new();
    }

    let desired_total = ((total_length / AUTOMOBILES_SPACING_M).round() as usize).max(1);
    let budget = desired_total.min(MAX_AUTOMOBILES_PER_TILE);

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

        // Proportional share of the tile's budget, so longer chains get more automobiles, capped
        // per-chain so one exceptionally long chain can't visually hog the whole tile's budget.
        let share = ((length / total_length) * budget as f32).round() as usize;
        let count = share
            .clamp(1, MAX_AUTOMOBILES_PER_CHAIN)
            .min(budget - assigned);

        for automobile_index in 0..count {
            let slot = instances.len();
            instances.push(build_instance(
                chain,
                coordinate,
                chain_index,
                automobile_index,
                slot,
            ));
        }
        assigned += count;
    }

    instances
}

/// Keeps one automobile-batch entity alive per terrain tile that is currently loaded at the highest
/// LOD and has at least one road chain, spawning/despawning entities as the active tile set
/// changes - mirrors `buildings::instances::update_building_batches`. The whole-globe
/// `RoadTileIndex` is built exactly once, the first time a `TileAtlas` and the `RoadNetwork`
/// asset are both available (tile boundaries at the fixed highest LOD never change afterward, so
/// there's nothing to rebuild).
pub fn update_automobiles_batches(
    mut commands: Commands,
    mut known: Local<HashMap<TileCoordinate, Option<Entity>>>,
    mut network_handle: Local<Option<Handle<RoadNetwork>>>,
    mut height_tile_handles: Local<HashMap<TileCoordinate, Handle<Image>>>,
    mut coarse_index: Local<Option<RoadCoarseIndex>>,
    asset_server: Res<AssetServer>,
    networks: Res<Assets<RoadNetwork>>,
    images: Res<Assets<Image>>,
    grids: Grids,
    terrain_query: Query<(Entity, &TileAtlas)>,
) {
    let network_handle = network_handle
        .get_or_insert_with(|| asset_server.load("earth/roads.ron"))
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
            // R32F file the terrain mesh itself displaces by - before placing automobiles on it.
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
                    AutomobilesTile(coordinate),
                    AutomobilesInstances(instances),
                    cell,
                    Transform::from_translation(translation),
                    ChildOf(root),
                ))
                .id();

            known.insert(coordinate, Some(entity));
        }
    }
}
