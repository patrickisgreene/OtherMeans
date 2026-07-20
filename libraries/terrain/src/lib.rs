//! This crate provides the ability to render beautiful height-field terrains of any size.
//! This is achieved in extensible and modular manner, so that the terrain data
//! can be accessed from nearly anywhere (systems, shaders) [^note].
//!
//! # Background
//! There are three critical questions that each terrain renderer has to solve:
//!
//! ## How to store, manage and access the terrain data?
//! Each terrain has different types of textures associated with it.
//! For example a simple one might only need height and albedo information.
//! Because terrains can be quite large the space required for all of these so called
//! attachments, can/should not be stored in RAM and VRAM all at once.
//! Thus they have to be streamed in and out depending on the positions of the
//! viewers (cameras, lights, etc.).
//! Therefore the terrain is subdivided into a giant tile_tree, whose tiles store their
//! section of these attachments.
//! This crate uses the chunked clipmap data structure, which consist of two pieces working together.
//! The wrapping [`TileTree`](prelude::TileTree) views together with
//! the [`TileAtlas`](prelude::TileAtlas) (the data structure
//! that stores all of the currently loaded data) can be used to efficiently retrieve
//! the best currently available data at any position for terrains of any size.
//! See the [`terrain_data`] module for more information.
//!
//! ## How to best approximate the terrain geometry?
//! Even a small terrain with a height map of 1000x1000 pixels would require 1 million vertices
//! to be rendered each frame per view, with an naive approach without an lod strategy.
//! To better distribute the vertices over the screen there exist many different algorithms.
//! This crate comes with its own default terrain geometry algorithm, called the
//! Uniform Distance-Dependent Level of Detail (UDLOD), which was developed with performance and
//! quality scalability in mind.
//! See the [`render`] module for more information.
//! You can also implement a different algorithm yourself and only use the terrain
//! data structures to solve the first question.
//!
//! ## How to shade the terrain?
//! The third and most important challenge of terrain rendering is the shading. This is a very
//! project specific problem and thus there does not exist a one-size-fits-all solution.
//! You can define your own terrain [Material](bevy::prelude::Material) and shader with all the
//! detail textures tailored to your application.
//! In the future this plugin will provide modular shader functions to make techniques like splat
//! mapping, triplane mapping, etc. easier.
//! Additionally a virtual texturing solution might be integrated to achieve better performance.
//!
//! [^note]: Some of these claims are not yet fully implemented.

pub mod config;
pub mod data;
pub mod debug;
pub mod formats;
pub mod math;
pub mod mipmap;
pub mod picking;
pub mod plugin;
pub mod render;
pub mod shaders;
pub mod util;
pub mod view;

#[doc(hidden)]
pub mod prelude {
    //! `use terrain::prelude::*;` to import common components, bundles, and plugins.

    pub use crate::{
        config::TerrainConfig,
        data::{AttachmentConfig, AttachmentFormat, AttachmentLabel, TileAtlas, TileTree},
        debug::{
            DebugCameraController, DebugTerrainMaterial, LoadingImages, OrbitalCameraController,
            TerrainDebugPlugin,
        },
        math::{TerrainShape, TileCoordinate},
        picking::{PickingData, TerrainPickingPlugin},
        plugin::{TerrainPlugin, TerrainSettings},
        render::TerrainMaterialPlugin,
        view::{TerrainViewComponents, TerrainViewConfig},
    };
    pub use big_space::{commands::BigSpaceCommands, grid::Grid};
}

use std::marker::PhantomData;

use bevy::{app::PluginGroupBuilder, prelude::*};
use big_space::plugin::BigSpaceMinimalPlugins;

pub struct TerrainPlugins<M: Material> {
    _phantom: PhantomData<M>,
}

impl<M: Material + Clone> Default for TerrainPlugins<M>
where
    M::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<M: Material + Clone> PluginGroup for TerrainPlugins<M>
where
    M::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(prelude::TerrainPlugin)
            .add(prelude::TerrainDebugPlugin)
            .add(prelude::TerrainPickingPlugin)
            .add_group(BigSpaceMinimalPlugins)
            .add(prelude::TerrainMaterialPlugin::<M>::default())
    }
}
