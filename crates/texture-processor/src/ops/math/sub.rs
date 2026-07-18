use std::path::PathBuf;

use clap::Parser;

use crate::cli::Cli;
use crate::utils::{self, MaskArgs};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Sub {
    file1: PathBuf,
    file2: PathBuf,
    #[arg(long, short)]
    output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    format: TextureFormat,
    #[clap(flatten)]
    mask: MaskArgs,
}

/// Subtracts `file2` from `file1`, clamping at 0 (never wraps negative).
pub fn sub(global: Cli, args: Sub) -> Result<(), ProcessError> {
    match args.mask.resolve()? {
        Some(mask) => utils::run_binary_op_masked(
            &global,
            &args.file1,
            &args.file2,
            mask,
            &args.output,
            args.format,
            "sub",
            |a, b| a.saturating_sub(b),
        ),
        None => utils::run_binary_op(
            &global,
            &args.file1,
            &args.file2,
            &args.output,
            args.format,
            "sub",
            |a, b| a.saturating_sub(b),
        ),
    }
}
