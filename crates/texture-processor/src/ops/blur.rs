use std::fmt;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::cli::Cli;
use crate::utils::{self, MaskArgs};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Blur {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(short, long)]
    pub radius: u32,
    #[arg(long, default_value_t = BlurKernel::default())]
    pub kernel: BlurKernel,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    #[clap(flatten)]
    pub mask: MaskArgs,
}

/// The two supported separable blur kernels. `Gaussian` (the default)
/// derives its sigma from `radius` via the 3-sigma rule (`sigma =
/// radius/3`, so a radius-r kernel captures ~99.7% of the Gaussian's mass)
/// rather than exposing a separate `--sigma` flag, so `--radius` alone
/// controls both kernels' effective footprint consistently.
///
/// Note: this is unrelated to `image::imageops::FilterType::Gaussian` (used
/// by `scale`'s unrelated `--filter-type` option, see `ops/scale.rs`) —
/// that's a resampling filter for resizing, a different concept from this
/// convolution-kernel blur.
#[derive(Debug, PartialEq, Default, Clone, Copy, Hash, Eq, ValueEnum)]
pub enum BlurKernel {
    Box,
    #[default]
    Gaussian,
}

impl fmt::Display for BlurKernel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlurKernel::Box => write!(f, "box"),
            BlurKernel::Gaussian => write!(f, "gaussian"),
        }
    }
}

/// Generates a normalized (sums to 1.0), symmetric 1D convolution kernel of
/// length `2*radius+1`. `radius == 0` always returns `[1.0]` regardless of
/// `kernel` — a true no-op tap — which also sidesteps a 0/0 in the
/// Gaussian formula's sigma term rather than special-casing it away
/// elsewhere in the pipeline.
pub fn kernel_weights(kernel: BlurKernel, radius: u32) -> Vec<f32> {
    if radius == 0 {
        return vec![1.0];
    }
    let r = radius as i32;
    let raw: Vec<f32> = match kernel {
        BlurKernel::Box => vec![1.0; (2 * radius + 1) as usize],
        BlurKernel::Gaussian => {
            let sigma = radius as f32 / 3.0;
            (-r..=r)
                .map(|i| (-((i * i) as f32) / (2.0 * sigma * sigma)).exp())
                .collect()
        }
    };
    let sum: f32 = raw.iter().sum();
    raw.into_iter().map(|w| w / sum).collect()
}

pub fn blur(global: Cli, args: Blur) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    let weights = kernel_weights(args.kernel, args.radius);

    match args.mask.resolve()? {
        Some(mask) => {
            match utils::open_streamable_masked_blur_sources(&args.input, mask, args.format)? {
                Some((image_source, mask_source)) => utils::stream_masked_blur(
                    image_source,
                    mask_source,
                    mask.excludes_white,
                    &weights,
                    &args.output,
                    args.format,
                    global.display_format,
                    "blur",
                ),
                None => utils::buffered_masked_blur(
                    &args.input,
                    mask,
                    &args.output,
                    args.format,
                    global.no_memory_limit,
                    &weights,
                    global.display_format,
                    "blur",
                ),
            }
        }
        None => match utils::open_streamable_blur_source(&args.input, args.format)? {
            Some(source) => utils::stream_blur(
                source,
                &weights,
                &args.output,
                args.format,
                global.display_format,
                "blur",
            ),
            None => utils::buffered_blur(
                &args.input,
                &args.output,
                args.format,
                global.no_memory_limit,
                &weights,
                global.display_format,
                "blur",
            ),
        },
    }
}
