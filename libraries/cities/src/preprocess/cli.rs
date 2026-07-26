use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[arg(long = "populated-places")]
    pub populated_places: PathBuf,
    #[arg(long = "out-file")]
    pub out_file: PathBuf,
    #[arg(long = "city-lights-out-file")]
    pub city_lights_out_file: Option<PathBuf>,
    #[arg(long = "light-cluster-radius-km", default_value_t = 150.0)]
    pub light_cluster_radius_km: f64,
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
