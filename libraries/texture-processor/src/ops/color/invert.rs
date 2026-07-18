use clap::Parser;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::utils::{self, MaskArgs, invert_byte};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Invert {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    #[clap(flatten)]
    pub mask: MaskArgs,
}

pub fn invert(global: Cli, args: Invert) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    if let Some(mask) = args.mask.resolve()? {
        if args.format == TextureFormat::Png
            && utils::is_png(&args.input)?
            && utils::stream_png_pointwise_masked(
                &args.input,
                &args.output,
                true,
                invert_byte,
                mask,
                global.display_format,
                "invert",
            )?
        {
            return Ok(());
        }

        if args.format == TextureFormat::Tiff
            && utils::is_tiff(&args.input)?
            && utils::stream_tiff_pointwise_masked(
                &args.input,
                &args.output,
                true,
                invert_byte,
                mask,
                global.display_format,
                "invert",
            )?
        {
            return Ok(());
        }

        return utils::buffered_fallback_masked(
            &args.input,
            &args.output,
            args.format.into(),
            global.no_memory_limit,
            true,
            invert_byte,
            mask,
            global.display_format,
            "invert",
        );
    }

    if args.format == TextureFormat::Png
        && utils::is_png(&args.input)?
        && utils::stream_png_pointwise(
            &args.input,
            &args.output,
            true,
            invert_byte,
            global.display_format,
            "invert",
        )?
    {
        return Ok(());
    }

    if args.format == TextureFormat::Tiff
        && utils::is_tiff(&args.input)?
        && utils::stream_tiff_pointwise(
            &args.input,
            &args.output,
            true,
            invert_byte,
            global.display_format,
            "invert",
        )?
    {
        return Ok(());
    }

    let _progress = utils::Progress::new(global.display_format, "invert", None);
    utils::buffered_fallback(
        &args.input,
        &args.output,
        args.format.into(),
        global.no_memory_limit,
        image::imageops::invert,
    )
}
