use bevy::{
    math::{DVec2, DVec3},
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::sync_component::SyncComponent,
};
use big_space::prelude::{CellCoord, Grid, Grids};
use terrain::math::Coordinate;
use terrain::prelude::{TerrainShape, TileAtlas, TileCoordinate};

use crate::height::HeightMap;
use crate::ocean_mask::OceanMask;
use crate::population::PopulationDensity;

/// Number of building instances along each edge of an active tile.
pub const GRID_SIZE: u32 = 100;
/// Fraction of a building's footprint cell that the cube actually occupies, leaving gaps
/// between buildings rather than a solid slab. Height is derived from this too (see
/// `MIN_HEIGHT_FRACTION`/`MAX_HEIGHT_MULTIPLIER`), so this single constant scales whole
/// buildings uniformly.
const FOOTPRINT_FILL: f64 = 0.40;
const BUILDING_COLOR: [f32; 3] = [0.62, 0.58, 0.52];
/// Height (relative to footprint width) of a building in the least-populated non-zero areas.
const MIN_HEIGHT_FRACTION: f32 = 0.00001;
/// Additional height (relative to footprint width) at maximum population density, on top of
/// `MIN_HEIGHT_FRACTION`.
const MAX_HEIGHT_MULTIPLIER: f32 = 6.0;
/// Density byte (0-255, gamma-compressed) below which an area is treated as unpopulated. The
/// gamma curve used to produce the population texture (exponent 0.3) compresses low raw
/// population counts into surprisingly high byte values, so without this floor almost any
/// nonzero population — including near-empty rural areas — ends up spawning buildings. Tune
/// this by eye; higher values mean sparser (but still nonzero) areas get skipped too.
const MIN_DENSITY_BYTE: u8 = 80;
/// How much of a grid cell's free space (cell width minus footprint width) a building may be
/// nudged by, as a fraction in `[0, 1]`. Breaks up the otherwise perfectly regular placement
/// grid without letting buildings drift into a neighboring cell.
const JITTER_STRENGTH: f64 = 0.7;

/// Per-instance cube data for a single active tile, uploaded to the GPU as a vertex buffer.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    /// xyz = offset from the owning entity's tile-center position, w = X/Z footprint size.
    pub position_and_footprint: [f32; 4],
    /// Rotation quaternion (x, y, z, w) that aligns the cube's local +Y with the surface normal.
    pub rotation: [f32; 4],
    /// rgb = color, a = Y height (population-density-driven).
    pub color_and_height: [f32; 4],
}

/// Identifies which terrain tile a building batch entity belongs to.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildingTile(pub TileCoordinate);

/// Per-instance cube data for a single active tile. Only ever inserted once (at tile spawn,
/// never mutated afterward), so it's extracted into the render world via a hand-written
/// `Changed<>`-gated system (`render::draw::extract_building_instances`) instead of
/// `ExtractComponentPlugin`, which would otherwise re-clone this (potentially ~480KB at max
/// density) into the render world on every single frame regardless of whether it changed.
#[derive(Component, Clone)]
pub struct BuildingInstances(pub Vec<InstanceData>);

impl SyncComponent for BuildingInstances {
    type Target = BuildingInstances;
}

fn shape_is_spherical(shape: TerrainShape) -> bool {
    !matches!(shape, TerrainShape::Plane { .. })
}

/// Converts a point on the unit cube-sphere to (latitude, longitude) in degrees, using the same
/// convention as `terrain-preprocess`'s GDAL transformer
/// (`libraries/terrain-preprocess/src/core/transformers.rs`).
fn lat_lon_degrees(unit_position: DVec3) -> (f64, f64) {
    let lon = unit_position.z.atan2(-unit_position.x);
    let lat = unit_position.y.asin();
    (lat.to_degrees(), lon.to_degrees())
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

/// Deterministic pseudo-random UV-space offset for a grid sample point, stable across tile
/// respawns since it's derived purely from the tile/sample coordinates (no external RNG state).
fn jitter_uv(coordinate: TileCoordinate, ix: u32, iy: u32, amplitude: f64) -> DVec2 {
    let seed = mix(
        mix(coordinate.face as u64, coordinate.lod as u64),
        mix(
            mix(coordinate.xy.x as u64, coordinate.xy.y as u64),
            mix(ix as u64, iy as u64),
        ),
    );

    let hx = hash_u64(seed);
    let hy = hash_u64(seed ^ 0x9E3779B97F4A7C15);

    let jx = (hx as f64 / u64::MAX as f64) * 2.0 - 1.0;
    let jy = (hy as f64 / u64::MAX as f64) * 2.0 - 1.0;

    DVec2::new(jx, jy) * amplitude
}

/// Checks whether any of the 4 corners of a `half_uv`-radius square footprint centered at
/// `center_uv` fall in water, per `ocean_mask`.
fn footprint_touches_ocean(
    face: u32,
    center_uv: DVec2,
    half_uv: f64,
    spherical: bool,
    ocean_mask: &OceanMask,
) -> bool {
    for dy in [-1.0, 1.0] {
        for dx in [-1.0, 1.0] {
            let corner_uv = center_uv + DVec2::new(dx, dy) * half_uv;
            let corner_unit = Coordinate::new(face, corner_uv).unit_position(spherical);
            let (lat, lon) = lat_lon_degrees(corner_unit);
            if ocean_mask.is_water(lat, lon) {
                return true;
            }
        }
    }
    false
}

/// Generates a grid of mono-colored cube instances covering `coordinate`'s footprint, skipping
/// any sample point where `population` reports zero density or `ocean_mask` reports water, and
/// scaling each surviving instance's height by its local density and placing it at its actual
/// terrain elevation (via `height_map` and `height_scale`), plus the big_space cell/translation
/// the owning entity should be spawned at. Returns an empty instance list if the tile has no
/// buildings anywhere in its footprint.
///
/// This is the single place building placement happens, kept separate from tile discovery so
/// that future work (e.g. per-city population data) can replace it without touching the
/// surrounding ECS plumbing.
pub fn generate_tile_instances(
    coordinate: TileCoordinate,
    shape: TerrainShape,
    height_scale: f32,
    grid: &Grid,
    population: &PopulationDensity,
    ocean_mask: &OceanMask,
    height_map: &HeightMap,
) -> (CellCoord, Vec3, Vec<InstanceData>) {
    let spherical = shape_is_spherical(shape);
    let tile_count = 2f64.powi(coordinate.lod as i32);

    let center_uv = (coordinate.xy.as_dvec2() + 0.5) / tile_count;
    let center_world = Coordinate::new(coordinate.face, center_uv).local_position(shape, 0.0);
    let (cell, translation) = grid.translation_to_grid(center_world);

    let footprint = (shape.face_size() / tile_count / GRID_SIZE as f64 * FOOTPRINT_FILL) as f32;

    let cell_uv = 1.0 / (tile_count * GRID_SIZE as f64);
    let footprint_uv = cell_uv * FOOTPRINT_FILL;
    let half_footprint_uv = footprint_uv * 0.5;
    let jitter_amplitude = JITTER_STRENGTH * (cell_uv - footprint_uv) * 0.5;

    let mut instances = Vec::with_capacity((GRID_SIZE * GRID_SIZE) as usize);
    for iy in 0..GRID_SIZE {
        for ix in 0..GRID_SIZE {
            let grid_uv = (coordinate.xy.as_dvec2()
                + (DVec2::new(ix as f64, iy as f64) + 0.5) / GRID_SIZE as f64)
                / tile_count;
            let sample_uv = grid_uv + jitter_uv(coordinate, ix, iy, jitter_amplitude);

            let sample = Coordinate::new(coordinate.face, sample_uv);
            let unit_position = sample.unit_position(spherical);

            let (lat, lon) = lat_lon_degrees(unit_position);
            let density_byte = population.sample(lat, lon);
            if density_byte < MIN_DENSITY_BYTE {
                continue;
            }

            if footprint_touches_ocean(
                coordinate.face,
                sample_uv,
                half_footprint_uv,
                spherical,
                ocean_mask,
            ) {
                continue;
            }

            // Real terrain elevation, in the same units the terrain mesh itself displaces by
            // (see `libraries/buildings/src/height.rs` for why `height_scale` is needed here),
            // so buildings sit on the actual visible ground instead of the base ellipsoid.
            let elevation = height_map.sample(lat, lon) * height_scale;
            let world_position = shape.position_unit_to_local(unit_position, elevation as f64);

            let normal = if spherical {
                (shape.scale() * unit_position).normalize()
            } else {
                DVec3::Y
            };

            let density = density_byte as f32 / 255.0;
            let height = footprint * (MIN_HEIGHT_FRACTION + density * MAX_HEIGHT_MULTIPLIER);

            let offset =
                (world_position - center_world).as_vec3() + normal.as_vec3() * (height * 0.5);
            let rotation = Quat::from_rotation_arc(Vec3::Y, normal.as_vec3());

            instances.push(InstanceData {
                position_and_footprint: [offset.x, offset.y, offset.z, footprint],
                rotation: rotation.to_array(),
                color_and_height: [
                    BUILDING_COLOR[0],
                    BUILDING_COLOR[1],
                    BUILDING_COLOR[2],
                    height,
                ],
            });
        }
    }

    (cell, translation, instances)
}

/// Keeps one building-batch entity alive per terrain tile that is currently loaded at the
/// highest LOD and has at least one populated, non-ocean sample point, spawning/despawning
/// entities as the active tile set (and its population/ocean coverage) changes. Waits for the
/// population density, ocean mask, and height map assets to finish loading before doing
/// anything.
pub fn update_building_batches(
    mut commands: Commands,
    mut known: Local<HashMap<TileCoordinate, Option<Entity>>>,
    mut population_handle: Local<Option<Handle<PopulationDensity>>>,
    mut ocean_mask_handle: Local<Option<Handle<OceanMask>>>,
    mut height_map_handle: Local<Option<Handle<HeightMap>>>,
    asset_server: Res<AssetServer>,
    populations: Res<Assets<PopulationDensity>>,
    ocean_masks: Res<Assets<OceanMask>>,
    height_maps: Res<Assets<HeightMap>>,
    grids: Grids,
    terrain_query: Query<(Entity, &TileAtlas)>,
) {
    let population_handle = population_handle
        .get_or_insert_with(|| asset_server.load("earth/population.tif"))
        .clone();
    let Some(population) = populations.get(&population_handle) else {
        return;
    };

    let ocean_mask_handle = ocean_mask_handle
        .get_or_insert_with(|| asset_server.load("earth/ocean-mask.tif"))
        .clone();
    let Some(ocean_mask) = ocean_masks.get(&ocean_mask_handle) else {
        return;
    };

    let height_map_handle = height_map_handle
        .get_or_insert_with(|| asset_server.load("earth/height.tif"))
        .clone();
    let Some(height_map) = height_maps.get(&height_map_handle) else {
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
            let (cell, translation, instances) = generate_tile_instances(
                coordinate,
                tile_atlas.shape,
                tile_atlas.height_scale,
                grid,
                population,
                ocean_mask,
                height_map,
            );

            if instances.is_empty() {
                known.insert(coordinate, None);
                continue;
            }

            let entity = commands
                .spawn((
                    BuildingTile(coordinate),
                    BuildingInstances(instances),
                    cell,
                    Transform::from_translation(translation),
                    ChildOf(root),
                ))
                .id();

            known.insert(coordinate, Some(entity));
        }
    }
}
