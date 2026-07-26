use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[arg(long = "populated-places")]
    pub populated_places: PathBuf,
    #[arg(long = "out-file")]
    pub out_file: PathBuf,
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
