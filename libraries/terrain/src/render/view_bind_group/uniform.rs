use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

use crate::data::{TileTree, TileTreeEntry};
use crate::math::{TerrainViewport, ViewCoordinate};

#[derive(Default, ShaderType)]
pub struct TileTreeUniform {
    #[shader(size(runtime))]
    pub(crate) entries: Vec<TileTreeEntry>,
}

#[derive(ShaderType)]
pub(crate) struct TerrainViewUniform {
    tree_size: u32,
    geometry_tile_count: u32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    load_distance: f32,
    subdivision_distance: f32,
    morph_range: f32,
    blend_range: f32,
    precision_distance: f32,
    face: u32,
    lod: u32,
    coordinates: [ViewCoordinate; 6],
    world_position: Vec3,
    half_spaces: [Vec4; 6],
    viewport_shape: u32,
    surface_approximation: [crate::math::SurfaceApproximation; 6],
}

impl From<&TileTree> for TerrainViewUniform {
    fn from(tile_tree: &TileTree) -> Self {
        TerrainViewUniform {
            tree_size: tile_tree.tree_size,
            geometry_tile_count: tile_tree.geometry_tile_count,
            grid_size: tile_tree.grid_size as f32,
            vertices_per_row: 2 * (tile_tree.grid_size + 2),
            vertices_per_tile: 2 * tile_tree.grid_size * (tile_tree.grid_size + 2),
            morph_distance: tile_tree.morph_distance as f32,
            blend_distance: tile_tree.blend_distance as f32,
            load_distance: tile_tree.load_distance as f32,
            subdivision_distance: tile_tree.subdivision_distance as f32,
            precision_distance: tile_tree.precision_distance as f32,
            viewport_shape: match tile_tree.viewport {
                TerrainViewport::Sphere => 0,
                TerrainViewport::Square => 1,
            },
            morph_range: tile_tree.morph_range,
            blend_range: tile_tree.blend_range,
            face: tile_tree.view_face,
            lod: tile_tree.view_lod,
            coordinates: tile_tree
                .view_coordinates
                .map(|view_coordinate| ViewCoordinate::new(view_coordinate, tile_tree.view_lod)),
            world_position: tile_tree.view_world_position,
            half_spaces: tile_tree.half_spaces,

            surface_approximation: tile_tree.surface_approximation,
        }
    }
}
