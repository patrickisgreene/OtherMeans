use std::marker::PhantomData;

use bevy::{
    pbr::MeshPipelineSystems,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems, render_phase::AddRenderCommand,
        render_resource::SpecializedRenderPipelines,
    },
};

use crate::{
    render::{
        TerrainItem,
        material::{
            DrawTerrain, TerrainRenderPipeline, init_terrain_render_pipeline, queue_terrain,
        },
    },
    spawn::{TerrainsToSpawn, spawn_terrains},
};

/// This plugin adds a custom material for a terrain.
///
/// It can be used to render the terrain using a custom vertex and fragment shader.
pub struct TerrainMaterialPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Default for TerrainMaterialPlugin<M> {
    fn default() -> Self {
        Self(default())
    }
}

impl<M: Material + Clone> Plugin for TerrainMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<M>::default())
            .insert_resource(TerrainsToSpawn::<M>(vec![]))
            .add_systems(PostUpdate, spawn_terrains::<M>);

        app.sub_app_mut(RenderApp)
            .add_render_command::<TerrainItem, DrawTerrain>()
            .init_resource::<SpecializedRenderPipelines<TerrainRenderPipeline<M>>>()
            .add_systems(
                RenderStartup,
                init_terrain_render_pipeline::<M>.after(MeshPipelineSystems),
            )
            .add_systems(
                Render,
                queue_terrain::<M>.in_set(RenderSystems::QueueMeshes),
            );
    }
}
