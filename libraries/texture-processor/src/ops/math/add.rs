use std::path::PathBuf;

use clap::Parser;

use crate::cli::Cli;
use crate::utils::{self, MaskArgs};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Add {
    file1: PathBuf,
    file2: PathBuf,
    #[arg(long, short)]
    output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    format: TextureFormat,
    #[clap(flatten)]
    mask: MaskArgs,
}

pub fn add(global: Cli, args: Add) -> Result<(), ProcessError> {
    match args.mask.resolve()? {
        Some(mask) => utils::run_binary_op_masked(
            &global,
            &args.file1,
            &args.file2,
            mask,
            &args.output,
            args.format,
            "add",
            |a, b| a.saturating_add(b),
        ),
        None => utils::run_binary_op(
            &global,
            &args.file1,
            &args.file2,
            &args.output,
            args.format,
            "add",
            |a, b| a.saturating_add(b),
        ),
    }
}
