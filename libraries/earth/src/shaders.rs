use bevy::prelude::*;

/// Bevy only auto-loads a shader's `#import "quoted/path.wgsl"` dependencies - the bare
/// `#import earth::module::Item` style (used for everything but `fragment.wgsl`, which is
/// loaded directly as the material's `fragment_shader()`) never gets fetched by anything on its
/// own, so these modules have to be force-loaded up front or their imports silently fail to
/// resolve. Mirrors `terrain::shaders::InternalShaders`. The handles just need to stay alive
/// for the shaders to remain loaded; nothing reads them back out.
#[derive(Default, Resource)]
pub(crate) struct InternalShaders(#[allow(dead_code)] Vec<Handle<Shader>>);

pub(crate) fn load_earth_shaders(app: &mut App) {
    let handles = [
        "shaders/earth/bindings.wgsl",
        "shaders/earth/render.wgsl",
        "shaders/earth/types/params.wgsl",
        "shaders/earth/ocean/compute.wgsl",
        "shaders/earth/ocean/triplanar_wave.wgsl",
    ]
    .into_iter()
    .map(|path| {
        app.world_mut()
            .resource_mut::<AssetServer>()
            .load::<Shader>(path)
    })
    .collect();

    app.insert_resource(InternalShaders(handles));
}
