mod default_plugins;
mod example_cli;

use bevy::math::DVec3;
pub use default_plugins::*;
pub use example_cli::ExampleCli;

pub const EARTH_RADIUS: f64 = 6_371_000.0;

use std::path::{Path, PathBuf};

pub fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

/// Converts a latitude/longitude (degrees) to a point on the unit cube-sphere - inverse of
/// `buildings::instances::lat_lon_degrees`.
pub fn lat_lon_to_unit_position(lat_deg: f64, lon_deg: f64) -> DVec3 {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let y = lat.sin();
    let horizontal = lat.cos();
    DVec3::new(-horizontal * lon.cos(), y, horizontal * lon.sin())
}
