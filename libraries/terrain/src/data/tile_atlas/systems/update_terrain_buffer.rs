use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{data::TileAtlas, render::TerrainUniform};

pub fn update_terrain_buffer(
    mut tile_atlases: Query<(&mut TileAtlas, &GlobalTransform), Changed<GlobalTransform>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    for (tile_atlas, global_transform) in &mut tile_atlases {
        let mut terrain_buffer = buffers.get_mut(&tile_atlas.terrain_buffer).unwrap();
        terrain_buffer.set_data(TerrainUniform::new(&tile_atlas, global_transform));
    }
}
