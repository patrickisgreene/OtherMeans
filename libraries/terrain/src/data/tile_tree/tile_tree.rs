use crate::{
    config::TerrainConfig,
    math::{Coordinate, TerrainShape, TerrainViewport, TileCoordinate},
    render::TerrainViewUniform,
    view::TerrainViewConfig,
};
use bevy::{
    asset::RenderAssetUsages,
    math::{DVec2, DVec3},
    prelude::*,
    render::{
        gpu_readback::Readback,
        render_resource::{BufferUsages, ShaderType},
        storage::ShaderBuffer,
    },
};
use itertools::iproduct;
use ndarray::Array4;
use std::cmp::Ordering;

/// A quadtree-like view of a terrain, that requests and releases tiles from the [`TileAtlas`]
/// depending on the distance to the viewer.
///
/// It can be used to access the best currently loaded tile of the [`TileAtlas`].
/// Additionally its sends this data to the GPU via the
/// [`GpuTileTree`](super::gpu_tile_tree::GpuTileTree) so that it can be utilised
/// in shaders as well.
///
/// Each view (camera, shadow-casting light) that should consider the terrain has to
/// have an associated tile tree.
///
/// This tile tree is a "cube" with a size of (`tree_size`x`tree_size`x`lod_count`), where each layer
/// corresponds to a lod. These layers are wrapping (modulo `tree_size`), that means that
/// the tile tree is always centered under the viewer and only considers `tree_size` / 2 tiles
/// in each direction.
///
/// Each frame the tile tree determines the state of each tile via the
/// `compute_requests` methode.
/// After the [`TileAtlas`] has adjusted to these requests, the tile tree retrieves the best
/// currently loaded tiles from the tile atlas via the `adjust` methode, which can later be used to access the terrain data.
#[derive(Component)]
pub struct TileTree {
    /// The current cpu tile_tree data. This is synced each frame with the gpu tile_tree data.
    pub data: Array4<super::TileTreeEntry>,
    /// Tiles that are no longer required by this tile_tree.
    pub released_tiles: Vec<TileCoordinate>,
    /// Tiles that are requested to be loaded by this tile_tree.
    pub requested_tiles: Vec<TileCoordinate>,
    /// The internal tile states of the tile_tree.
    pub tiles: Array4<super::TileState>,
    /// The count of tiles in x and y direction per layer.
    pub tree_size: u32,
    pub lod_count: u32,
    pub shape: TerrainShape,
    pub viewport: TerrainViewport,
    pub geometry_tile_count: u32,
    pub refinement_count: u32,
    pub grid_size: u32,
    pub morph_range: f32,
    pub blend_range: f32,
    pub morph_distance: f64,
    pub blend_distance: f64,
    pub subdivision_distance: f64,
    pub load_distance: f64,
    pub precision_distance: f64,
    pub view_face: u32,
    pub view_lod: u32,
    pub view_local_position: DVec3,
    pub view_world_position: Vec3,
    pub view_coordinates: [Coordinate; 6],
    pub half_spaces: [Vec4; 6],
    pub surface_approximation: [crate::math::SurfaceApproximation; 6],
    pub approximate_height: f32,
    pub order: u32,

    /// Set whenever `update` ran this frame (i.e. the view moved). Gates recomputation of data
    /// that depends continuously on the view position (surface approximation, view uniform).
    /// Consumed and reset by the downstream systems in the `PostUpdate` chain.
    pub(crate) dirty: bool,
    /// Set whenever `update` actually reassigned a tile's coordinate or flipped a request/release
    /// state (i.e. the view crossed into a different tile/LOD cell), or a tile finished loading.
    /// Gates the more expensive tile-grid rewrite and its GPU upload, which only depend on the
    /// discrete tile assignment, not continuous view position.
    pub(crate) tiles_dirty: bool,

    pub tile_tree_buffer: Handle<ShaderBuffer>,
    pub terrain_view_buffer: Handle<ShaderBuffer>,
    pub approximate_height_buffer: Handle<ShaderBuffer>,
}

impl TileTree {
    /// Creates a new tile_tree from a terrain and a terrain view config.
    pub fn new(
        config: &TerrainConfig,
        view_config: &TerrainViewConfig,
        terrain_view: (Entity, Entity),
        commands: &mut Commands,
        buffers: &mut Assets<ShaderBuffer>, // Todo: solve this dependency with a component hook in the future
    ) -> Self {
        let data = Array4::default((
            config.shape.face_count() as usize,
            config.lod_count as usize,
            view_config.tree_size as usize,
            view_config.tree_size as usize,
        ));

        let terrain_view_buffer = buffers.add(ShaderBuffer::with_size(
            TerrainViewUniform::min_size().get() as usize,
            RenderAssetUsages::all(),
        ));
        let tile_tree_buffer = buffers.add(ShaderBuffer::with_size(
            data.len() * size_of::<super::TileTreeEntry>(),
            RenderAssetUsages::all(),
        ));

        let mut approximate_height_buffer = ShaderBuffer::from(0.0);
        approximate_height_buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
        let approximate_height_buffer = buffers.add(approximate_height_buffer);

        commands
            .spawn((
                super::TerrainViewKey(terrain_view),
                Readback::buffer(approximate_height_buffer.clone()),
            ))
            .observe(super::approximate_height_readback);

        let face_size = config.shape.face_size();

        Self {
            tree_size: view_config.tree_size,
            lod_count: config.lod_count,
            shape: config.shape,
            viewport: view_config.viewport,
            geometry_tile_count: view_config.geometry_tile_count,
            refinement_count: view_config.refinement_count,
            grid_size: view_config.grid_size,
            morph_distance: view_config.morph_distance * face_size,
            blend_distance: view_config.blend_distance * face_size,
            load_distance: view_config.blend_distance
                * face_size
                * (1.0 + view_config.load_tolerance),
            subdivision_distance: view_config.morph_distance
                * face_size
                * (1.0 + view_config.subdivision_tolerance),
            morph_range: view_config.morph_range,
            blend_range: view_config.blend_range,
            precision_distance: view_config.precision_distance * config.shape.scale_scalar(),
            view_face: 0,
            view_lod: view_config.view_lod,
            view_local_position: default(),
            view_world_position: default(),
            data,
            tiles: Array4::default((
                config.shape.face_count() as usize,
                config.lod_count as usize,
                view_config.tree_size as usize,
                view_config.tree_size as usize,
            )),
            released_tiles: default(),
            requested_tiles: default(),
            view_coordinates: default(),
            half_spaces: default(),

            surface_approximation: default(),
            approximate_height: 0.0,
            order: view_config.order,
            dirty: false,
            tiles_dirty: false,
            tile_tree_buffer,
            terrain_view_buffer,
            approximate_height_buffer,
        }
    }

    fn compute_tree_xy(coordinate: Coordinate, tile_count: f64) -> DVec2 {
        // scale and clamp the coordinate to the tile tree bounds
        (coordinate.uv * tile_count).min(DVec2::splat(tile_count - 0.000001))
    }

    fn compute_origin(&self, view_coordinate: Coordinate, lod: u32) -> IVec2 {
        let tile_count = (lod as f64).exp2();
        let tree_xy = Self::compute_tree_xy(view_coordinate, tile_count);

        (tree_xy - 0.5 * self.tree_size as f64)
            .round()
            .clamp(
                DVec2::splat(0.0),
                DVec2::splat(tile_count - self.tree_size as f64),
            )
            .as_ivec2()
    }

    fn compute_tile_distance(&self, tile: TileCoordinate, view_coordinate: Coordinate) -> f64 {
        let tile_count = (tile.lod as f64).exp2();
        let view_tile_xy = Self::compute_tree_xy(view_coordinate, tile_count);
        let tile_offset = view_tile_xy.as_ivec2() - tile.xy;
        let mut offset = view_tile_xy % 1.0;

        offset.x = match tile_offset.x.cmp(&0) {
            Ordering::Less => 0.0,
            Ordering::Greater => 1.0,
            Ordering::Equal => offset.x,
        };

        offset.y = match tile_offset.y.cmp(&0) {
            Ordering::Less => 0.0,
            Ordering::Greater => 1.0,
            Ordering::Equal => offset.y,
        };

        let tile_local_position =
            Coordinate::new(tile.face, (tile.xy.as_dvec2() + offset) / tile_count)
                .local_position(self.shape, self.approximate_height);

        self.viewport
            .distance(tile_local_position, self.view_local_position)
    }

    /// Recomputes tile requests/releases for the current view position. Returns whether any
    /// tile's coordinate assignment or request/release state actually changed, so callers can
    /// skip downstream work (tile-grid rewrite, GPU upload) that only depends on the discrete
    /// tile assignment rather than the continuous view position.
    pub fn update(&mut self) -> bool {
        let mut changed = false;
        let view_coordinate = Coordinate::from_local_position(self.view_local_position, self.shape);
        self.view_face = view_coordinate.face;

        for face in 0..self.shape.face_count() {
            let view_coordinate = view_coordinate.project_to_face(face);
            self.view_coordinates[face as usize] = view_coordinate;

            for lod in 0..self.lod_count {
                let origin = self.compute_origin(view_coordinate, lod);

                for (x, y) in iproduct!(0..self.tree_size, 0..self.tree_size) {
                    let tile_coordinate = TileCoordinate {
                        face,
                        lod,
                        xy: origin + IVec2::new(x as i32, y as i32),
                    };

                    let tile_distance =
                        self.compute_tile_distance(tile_coordinate, view_coordinate);
                    let load_distance = self.load_distance / (tile_coordinate.lod as f64).exp2();

                    let state = if lod == 0 || tile_distance < load_distance {
                        super::RequestState::Requested
                    } else {
                        super::RequestState::Released
                    };

                    let tile = &mut self.tiles[[
                        face as usize,
                        lod as usize,
                        tile_coordinate.xy.x as usize % self.tree_size as usize,
                        tile_coordinate.xy.y as usize % self.tree_size as usize,
                    ]];

                    // check if tile_tree slot refers to a new tile
                    if tile_coordinate != tile.coordinate {
                        // release old tile
                        if tile.state == super::RequestState::Requested {
                            tile.state = super::RequestState::Released;
                            self.released_tiles.push(tile.coordinate);
                        }

                        tile.coordinate = tile_coordinate;
                        changed = true;
                    }

                    // request or release tile based on its distance to the view
                    match (tile.state, state) {
                        (super::RequestState::Released, super::RequestState::Requested) => {
                            tile.state = super::RequestState::Requested;
                            self.requested_tiles.push(tile.coordinate);
                            changed = true;
                        }
                        (super::RequestState::Requested, super::RequestState::Released) => {
                            tile.state = super::RequestState::Released;
                            self.released_tiles.push(tile.coordinate);
                            changed = true;
                        }
                        (_, _) => {}
                    }
                }
            }
        }

        changed
    }
}
