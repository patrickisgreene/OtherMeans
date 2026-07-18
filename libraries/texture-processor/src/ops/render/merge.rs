use clap::Parser;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::utils::{self, MaskSpec};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct MaskMerge {
    #[arg(short, long)]
    pub mask: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    pub file1: PathBuf,
    pub file2: PathBuf,
}

/// Selects, per pixel, `file1` where the mask is black (0) and `file2`
/// where it's white (1) - a hard-edged cutout/swap rather than a blend.
/// Reuses the same masked-binary-op machinery as `add`/`sub`/`mul`: a
/// `--black-mask`-style mask (`excludes_white: false`) already passes
/// through `file1` unchanged on black and applies `combine` on white, so
/// making `combine` just return `file2`'s byte gives exactly this
/// selection with no new blending/streaming logic needed.
pub fn mask_merge(global: Cli, args: MaskMerge) -> Result<(), ProcessError> {
    utils::run_binary_op_masked(
        &global,
        &args.file1,
        &args.file2,
        MaskSpec {
            path: &args.mask,
            excludes_white: false,
        },
        &args.output,
        args.format,
        "mask-merge",
        |_file1_byte, file2_byte| file2_byte,
    )
}
