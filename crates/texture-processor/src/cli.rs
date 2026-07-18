use clap::{Parser, Subcommand};

use crate::format::DisplayFormat;

#[derive(Parser, Clone)]
pub struct Cli {
    /// Overwrite the file if it exists already
    #[arg(short, long, default_value_t = false)]
    pub overwrite: bool,

    /// Disasble the memory limit protection against decompression attacks.
    #[arg(long = "no-memory-limit", default_value_t = false)]
    pub no_memory_limit: bool,

    /// The method used to view the results.
    #[arg(short, long = "display-format", default_value_t = DisplayFormat::default())]
    pub display_format: DisplayFormat,

    #[clap(subcommand)]
    pub command: Command,
}

impl Default for Cli {
    fn default() -> Self {
        Cli::parse()
    }
}

#[derive(Subcommand, Clone)]
pub enum Command {
    /// Combine 2 images using the '+' operation.
    Add(crate::ops::Add),
    /// Combine 2 images using the '-' operation.
    Sub(crate::ops::Sub),
    /// Combine 2 images using the '*' operation.
    Mul(crate::ops::Mul),
    /// Invert the colors on the given image.
    Invert(crate::ops::Invert),
    /// Run a contrast filter on the given image.
    Contrast(crate::ops::Contrast),
    /// Run a brightness filter on the given image.
    Brightness(crate::ops::Brightness),
    /// Resize an image.
    ScaleImage(crate::ops::ScaleImage),
    /// Combine images using specific channels in one image to specific channels of the resul image.
    RgbaConcat(crate::ops::RgbaConcat),
    /// Run a blur filter on an image.
    Blur(crate::ops::Blur),
    /// Combine two images based on a mask.
    MaskMerge(crate::ops::MaskMerge),
    /// Run a flood fill algorythm on an image.
    FloodFill(crate::ops::FloodFill),
    /// Generate a distance field from shapefiles.
    DistanceField(crate::distance::DistanceField),
    /// Render the contents of a shapefile to an image.
    RenderShapefile(crate::ops::RenderShapefile),
}
