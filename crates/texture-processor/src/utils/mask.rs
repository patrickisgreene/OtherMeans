use std::path::{Path, PathBuf};

use clap::Parser;

use crate::ProcessError;

// CLI flags shared by every op that supports an optional black & white mask
// image: `--white-mask <path>` / `--black-mask <path>`, mutually exclusive,
// selecting which color in the mask counts as "excluded". Flatten this into
// an op's `Parser` struct with `#[clap(flatten)]`.
//
// Deliberately a plain comment, not a doc comment: clap's derive surfaces a
// struct's doc comment as its `--help` description, and since this struct
// is always flattened (never used as its own top-level command), a `///`
// here would leak into every op's `--help` output as if it were that op's
// own description.
#[derive(Parser, Clone, Default)]
pub struct MaskArgs {
    /// A black & white mask image (same dimensions as the input(s)) whose
    /// WHITE pixels are excluded from this operation — left unmodified in
    /// the output (and, for `blur`, also excluded from contributing to
    /// their neighbors). Mutually exclusive with `--black-mask`.
    #[arg(long)]
    pub white_mask: Option<PathBuf>,
    /// Same as `--white-mask`, but BLACK pixels are the excluded ones.
    #[arg(long)]
    pub black_mask: Option<PathBuf>,
}

/// A resolved mask: which file, and which of its two colors is excluded.
#[derive(Clone, Copy)]
pub struct MaskSpec<'a> {
    pub path: &'a Path,
    pub excludes_white: bool,
}

impl MaskArgs {
    /// Resolves the two flags into a single `MaskSpec`, or `None` if
    /// neither was given. Errors if both were (ambiguous).
    pub fn resolve(&self) -> Result<Option<MaskSpec<'_>>, ProcessError> {
        match (&self.white_mask, &self.black_mask) {
            (Some(_), Some(_)) => Err(ProcessError::InvalidInput(
                "`--white-mask` and `--black-mask` are mutually exclusive".to_string(),
            )),
            (Some(path), None) => Ok(Some(MaskSpec {
                path,
                excludes_white: true,
            })),
            (None, Some(path)) => Ok(Some(MaskSpec {
                path,
                excludes_white: false,
            })),
            (None, None) => Ok(None),
        }
    }
}

/// Whether a mask byte counts as "valid" (not excluded), given which color
/// was designated the mask color. Threshold is the byte midpoint, robust
/// regardless of whether the mask is a true 0/255 bilevel image or
/// something close to it.
pub fn mask_byte_valid(byte: u8, excludes_white: bool) -> bool {
    (byte >= 128) != excludes_white
}

/// Reduces one decoded mask row to a per-pixel validity flag using only
/// the first channel — which for a grayscale mask *is* its one channel,
/// and for an RGB/RGBA mask is R, consistent either way since a true
/// black/white mask has every channel equal.
pub fn mask_row_to_valid(mask_row: &[u8], mask_channels: usize, excludes_white: bool) -> Vec<bool> {
    mask_row
        .chunks(mask_channels)
        .map(|px| mask_byte_valid(px[0], excludes_white))
        .collect()
}
