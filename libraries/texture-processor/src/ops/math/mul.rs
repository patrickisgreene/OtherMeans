use std::path::PathBuf;

use clap::Parser;

use crate::cli::Cli;
use crate::utils::{self, MaskArgs, multiply_byte};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Mul {
    file1: PathBuf,
    file2: PathBuf,
    #[arg(long, short)]
    output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    format: TextureFormat,
    #[clap(flatten)]
    mask: MaskArgs,
}

/// Multiplies `file1` and `file2` using the standard image-compositing
/// "multiply" formula — bytes are treated as fixed-point values in 0..=1
/// (0 = 0.0, 255 = 1.0) rather than raw integers. A plain `saturating_mul`
/// would saturate to 255 for almost any non-trivial pair of bytes (e.g.
/// 16 * 16 already overflows a `u8`), which isn't useful for combining
/// textures; this formula instead darkens (255 acts as an identity, 0
/// yields 0), matching what "multiply" means in image editors and when
/// combining a mask/AO map with a color map.
pub fn mul(global: Cli, args: Mul) -> Result<(), ProcessError> {
    match args.mask.resolve()? {
        Some(mask) => utils::run_binary_op_masked(
            &global,
            &args.file1,
            &args.file2,
            mask,
            &args.output,
            args.format,
            "mul",
            multiply_byte,
        ),
        None => utils::run_binary_op(
            &global,
            &args.file1,
            &args.file2,
            &args.output,
            args.format,
            "mul",
            multiply_byte,
        ),
    }
}
