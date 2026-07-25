use bevy::prelude::*;
use earth::EarthMaterial;
use terrain::TerrainConfigHandle;

use crate::Earth;

pub fn spawn_earth(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<EarthMaterial>>,
    query: Query<Entity, With<Earth>>,
) {
    let material = materials.add(EarthMaterial::new(&assets));

    commands.entity(query.single().unwrap()).insert((
        MeshMaterial3d(material),
        TerrainConfigHandle(assets.load("earth/terrain.ron")),
    ));
}
