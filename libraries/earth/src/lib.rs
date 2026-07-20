mod atmosphere;
pub mod debug;
mod material;
mod render_mode;
mod render_params;
mod shaders;

pub use atmosphere::*;
pub use material::*;
pub use render_mode::*;
pub use render_params::*;

use bevy::prelude::*;
use terrain::{TerrainPlugins, plugin::TerrainSettings};

pub struct EarthPlugin;

impl Plugin for EarthPlugin {
    fn build(&self, app: &mut App) {
        shaders::load_earth_shaders(app);

        // atlas_size sets how many distinct tiles can be resident at once *and* how many layers
        // each attachment's GPU texture array allocates - it's a genuine memory/zoom-depth
        // tradeoff, not just a size knob. The 1028 default (tuned for a single attachment) still
        // exceeds this machine's available VRAM even after shrinking light/dark to texture_size
        // 256, so this targets roughly the same total footprint as the smallest atlas_size that
        // ran cleanly, just spread over more (now smaller) tiles.
        let terrain_settings = TerrainSettings::new(vec!["earth"]);
        //terrain_settings.atlas_size = 512;

        app.add_plugins((
            atmosphere::EarthAtmospherePlugin,
            TerrainPlugins::<EarthMaterial>::default(),
        ))
        .insert_resource(terrain_settings)
        .add_systems(Update, toggle_render_mode)
        .insert_resource(ClearColor(Color::srgb(0.012, 0.012, 0.031)));
    }
}

fn toggle_render_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<EarthMaterial>>,
) {
    if input.just_pressed(KeyCode::F11) {
        for (_, material) in materials.iter_mut() {
            material.render_mode += 1;
            if material.render_mode == 7 {
                material.render_mode = 0;
            }
        }
    }
}
