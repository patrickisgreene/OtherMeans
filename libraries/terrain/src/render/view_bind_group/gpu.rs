use bevy::{
    ecs::component::Component,
    render::{
        render_resource::{BindGroup, Buffer, BufferDescriptor, BufferUsages, ShaderType},
        renderer::RenderDevice,
        sync_component::SyncComponent,
    },
};

use crate::{
    data::TileTree,
    math::TileCoordinate,
    render::{
        GeometryTile, Indirect, IndirectBindGroup, PrepassState, PrepassViewBindGroup,
        TerrainViewBindGroup,
    },
};

#[derive(Component)]
pub struct GpuTerrainView {
    pub order: u32,
    pub refinement_count: u32,
    pub indirect_buffer: Buffer,
    pub indirect_bind_group: Option<BindGroup>,
    pub prepass_view_bind_group: Option<BindGroup>,
    pub terrain_view_bind_group: Option<BindGroup>,

    pub indirect: IndirectBindGroup,
    pub prepass_view: PrepassViewBindGroup,
    pub terrain_view: TerrainViewBindGroup,
}

impl GpuTerrainView {
    pub fn new(device: &RenderDevice, tile_tree: &TileTree) -> Self {
        // Todo: figure out a better way of limiting the tile buffer size

        let tiles = device.create_buffer(&BufferDescriptor {
            label: None,
            size: GeometryTile::min_size().get() * tile_tree.geometry_tile_count as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let temporary_tiles = device.create_buffer(&BufferDescriptor {
            label: None,
            size: TileCoordinate::min_size().get() * tile_tree.geometry_tile_count as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let state = device.create_buffer(&BufferDescriptor {
            label: None,
            size: PrepassState::min_size().get(),
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let indirect = device.create_buffer(&BufferDescriptor {
            label: None,
            size: Indirect::min_size().get(),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let prepare_prepass = IndirectBindGroup {
            indirect: indirect.clone(),
        };
        let refine_tiles = PrepassViewBindGroup {
            terrain_view: tile_tree.terrain_view_buffer.clone(),
            approximate_height: tile_tree.approximate_height_buffer.clone(),
            tile_tree: tile_tree.tile_tree_buffer.clone(),
            final_tiles: tiles.clone(),
            temporary_tiles,
            state,
        };
        let terrain_view = TerrainViewBindGroup {
            terrain_view: tile_tree.terrain_view_buffer.clone(),
            approximate_height: tile_tree.approximate_height_buffer.clone(),
            tile_tree: tile_tree.tile_tree_buffer.clone(),
            geometry_tiles: tiles,
        };

        Self {
            order: tile_tree.order,
            refinement_count: tile_tree.refinement_count,
            indirect_buffer: indirect,
            indirect: prepare_prepass,
            prepass_view: refine_tiles,
            terrain_view,
            indirect_bind_group: None,
            prepass_view_bind_group: None,
            terrain_view_bind_group: None,
        }
    }
}

impl SyncComponent for TileTree {
    type Target = GpuTerrainView;
}
