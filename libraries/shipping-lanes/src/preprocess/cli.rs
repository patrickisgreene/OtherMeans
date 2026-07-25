use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[arg(long = "roads")]
    pub roads: PathBuf,
    #[arg(long = "out-file")]
    pub out_file: PathBuf,
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
