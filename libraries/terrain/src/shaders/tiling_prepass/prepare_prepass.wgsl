#import terrain::types::{TileCoordinate, Blend}
#import terrain::bindings::{terrain_view, approximate_height, temporary_tiles, state, indirect_buffer}
#import terrain::functions::{compute_view_coordinate, compute_world_coordinate, lookup_tile, apply_height, compute_blend}
#import terrain::attachments::{sample_height, sample_height_mask}

@compute @workgroup_size(1, 1, 1)
fn prepare_root() {
    state.counter = -1;
    atomicStore(&state.child_index, i32(terrain_view.geometry_tile_count - 1u));
    atomicStore(&state.final_index, 0);
    indirect_buffer.workgroup_count = vec3<u32>(1u, 1u, 1u);

#ifdef SPHERICAL
    // The face directly opposite the viewer is always beyond the horizon (its closest point is
    // ~135 deg from nadir, farther than the max possible horizon angle of 90 deg), so skip seeding it.
    let opposite_face = (terrain_view.face + 3u) % 6u;

    var count: u32 = 0u;
    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        if (i == opposite_face) { continue; }
        temporary_tiles[count] = TileCoordinate(i, 0u, vec2<u32>(0u));
        count = count + 1u;
    }
    state.tile_count = count;
#else
    temporary_tiles[0] = TileCoordinate(0u, 0u, vec2<u32>(0u));
    state.tile_count = 1u;
#endif

    // compute approximate height
    let coordinate       = compute_view_coordinate(terrain_view.face, terrain_view.lod);
    let world_coordinate = compute_world_coordinate(coordinate);
    let blend            = compute_blend(world_coordinate.view_distance);

    let tile = lookup_tile(coordinate, blend);
    if (!sample_height_mask(tile)) { approximate_height = sample_height(tile); }

    // Todo: this does not work
    // let distance = dot(normalize(apply_height(world_coordinate, approximate_height) - terrain_view.world_position), world_coordinate.normal);
    // if (distance > 0.0) { state.tile_count   = 0u; }
}

@compute @workgroup_size(1, 1, 1)
fn prepare_next() {
    if (state.counter == 1) {
        state.tile_count = u32(atomicExchange(&state.child_index, i32(terrain_view.geometry_tile_count - 1u)));
    }
    else {
        state.tile_count = terrain_view.geometry_tile_count - 1u - u32(atomicExchange(&state.child_index, 0));
    }

    state.counter = -state.counter;
    indirect_buffer.workgroup_count.x = (state.tile_count + 63u) / 64u;
}

@compute @workgroup_size(1, 1, 1)
fn prepare_render() {
    let tile_count = u32(atomicLoad(&state.final_index));
    let vertex_count = terrain_view.vertices_per_tile * tile_count;

    indirect_buffer.workgroup_count = vec3<u32>(vertex_count, 1u, 0u);
}
