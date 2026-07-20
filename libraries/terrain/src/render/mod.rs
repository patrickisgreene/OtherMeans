//! This module contains the implementation of the Uniform Distance-Dependent Level of Detail (UDLOD).
//!
//! This algorithm is responsible for approximating the terrain geometry.
//! Therefore tiny mesh tiles are refined in a tile_tree-like manner in a compute shader prepass for
//! each view. Then they are drawn using a single draw indirect call and morphed together to form
//! one continuous surface.

pub mod bind_group;
mod material;
pub mod pass;
pub mod tiling_prepass;
pub mod view_bind_group;

pub use self::{
    bind_group::GpuTerrain,
    material::TerrainMaterialPlugin,
    view_bind_group::{GpuTerrainView, TerrainViewBindGroup},
};

pub(crate) use self::{bind_group::*, pass::*, tiling_prepass::*, view_bind_group::*};
