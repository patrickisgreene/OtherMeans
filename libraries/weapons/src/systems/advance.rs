use crate::data::Weapon;
use crate::mission::{StrikeMission, StrikeState};
use bevy::{math::DVec3, prelude::*};
use big_space::prelude::*;

pub fn advance_strike_mission(
    time: Res<Time>,
    weapons: Res<Assets<Weapon>>,
    grids: Grids,
    mut strikes: Query<(
        Entity,
        &mut StrikeState,
        &StrikeMission,
        &mut Transform,
        Option<&mut CellCoord>,
    )>,
) {
    let earth_radius = 6_371_000.0;
    for (entity, mut state, mission, mut transform, cell_coord) in &mut strikes {
        let waypoints = &mission.waypoints;
        if waypoints.is_empty() || state.progress + 1 >= waypoints.len() {
            continue;
        }

        let speed = {
            let weapon_speed = weapons
                .get(&mission.package.weapon)
                .map(|w| w.movement.speed as f64);
            weapon_speed.unwrap_or(state.speed as f64)
        };

        let from_wp = &waypoints[state.progress];
        let to_wp = &waypoints[state.progress + 1];

        let from_unit = lat_lon_to_unit(from_wp.lat, from_wp.long);
        let to_unit = lat_lon_to_unit(to_wp.lat, to_wp.long);

        let arc_angle = from_unit.dot(to_unit).clamp(-1.0, 1.0).acos();
        let arc_length = arc_angle * earth_radius;

        let dt = if arc_length > 0.0 {
            time.delta_secs_f64() * speed / arc_length
        } else {
            1.0
        };

        state.segment_t = (state.segment_t + dt as f32).min(1.0);

        let world_pos = slerp_unit(from_unit, to_unit, state.segment_t as f64) * earth_radius;

        if let Some(grid) = grids.parent_grid(entity) {
            let (new_cell, new_translation) = grid.translation_to_grid(world_pos);
            transform.translation = new_translation;
            if let Some(mut cell) = cell_coord {
                *cell = new_cell;
            }
        } else {
            transform.translation = world_pos.as_vec3();
        }

        if state.segment_t >= 1.0 {
            state.progress += 1;
            state.segment_t = 0.0;
        }
    }
}
/// Converts geographic lat/lon (degrees) to a unit vector.
/// Convention: +Y north pole, +Z at lon=0 lat=0, +X at lon=90°E lat=0.
fn lat_lon_to_unit(lat_deg: f64, lon_deg: f64) -> DVec3 {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    DVec3::new(lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos())
}

/// Spherical linear interpolation between two unit vectors.
fn slerp_unit(a: DVec3, b: DVec3, t: f64) -> DVec3 {
    let dot = a.dot(b).clamp(-1.0, 1.0);
    let theta = dot.acos();
    if theta.abs() < 1e-10 {
        return a;
    }
    let sin_theta = theta.sin();
    (a * ((1.0 - t) * theta).sin() / sin_theta + b * (t * theta).sin() / sin_theta).normalize()
}
