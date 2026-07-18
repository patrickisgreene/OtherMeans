use clap::{Parser, ValueEnum};

use crate::utils::Progress;
use crate::{ProcessError, TextureFormat, cli::Cli};

use std::{fmt, fs::File, path::PathBuf};

#[derive(Parser, Clone)]
pub struct ScaleImage {
    #[arg(long)]
    pub width: u32,
    #[arg(long)]
    pub height: u32,
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long = "filter-type", default_value_t = FilterType::default())]
    pub filter_type: FilterType,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
}

#[derive(Debug, PartialEq, Default, Clone, Copy, Hash, Eq, ValueEnum)]
pub enum FilterType {
    Nearest,
    Triangle,
    CatmullRom,
    #[default]
    Gaussian,
    Lanczos3,
}

impl From<FilterType> for image::imageops::FilterType {
    fn from(value: FilterType) -> Self {
        match value {
            FilterType::Nearest => image::imageops::FilterType::Nearest,
            FilterType::Triangle => image::imageops::FilterType::Triangle,
            FilterType::CatmullRom => image::imageops::FilterType::CatmullRom,
            FilterType::Gaussian => image::imageops::FilterType::Gaussian,
            FilterType::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

impl fmt::Display for FilterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            FilterType::Nearest => write!(f, "nearest"),
            FilterType::Triangle => write!(f, "triangle"),
            FilterType::CatmullRom => write!(f, "catmullrom"),
            FilterType::Gaussian => write!(f, "gaussian"),
            FilterType::Lanczos3 => write!(f, "lanczos3"),
        }
    }
}

/// `image::imageops::resize` needs the whole decoded pixel buffer in memory.
/// general-purpose resize filters (Lanczos3, CatmullRom, etc.) require random
///  access across rows, so there's no way to keep decoded pixels off-heap while
/// still using `image`'s resize.
pub fn scale_image(global: Cli, args: ScaleImage) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    // `image::imageops::resize` is a single blocking call with no progress
    // callback, so unlike the row/strip-streamed ops there's no incremental
    // signal to report — just an indeterminate spinner for the duration.
    let _progress = Progress::new(global.display_format, "scale", None);

    let mut reader = image::ImageReader::open(args.input)?.with_guessed_format()?;
    if global.no_memory_limit {
        reader.no_limits();
    }
    let input_image = reader.decode()?;

    let mut file = File::create(args.output)?;

    image::imageops::resize(
        &input_image,
        args.width,
        args.height,
        image::imageops::FilterType::from(args.filter_type),
    )
    .write_to(&mut file, args.format.into())?;
    Ok(())
}
