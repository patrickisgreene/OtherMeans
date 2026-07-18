use clap::Parser;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::utils::{self, MaskArgs, Progress, contrast_transform};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Contrast {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(short, long, allow_hyphen_values = true)]
    pub contrast: f32,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    #[clap(flatten)]
    pub mask: MaskArgs,
}

pub fn contrast(global: Cli, args: Contrast) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    let contrast_byte = contrast_transform(args.contrast);

    // Unlike `invert`/`brighten_in_place`, `image::imageops::contrast_in_place`
    // applies its formula to every channel via a plain `map` — it does *not*
    // exempt alpha the way the other two ops do.
    if let Some(mask) = args.mask.resolve()? {
        if args.format == TextureFormat::Png
            && utils::is_png(&args.input)?
            && utils::stream_png_pointwise_masked(
                &args.input,
                &args.output,
                false,
                contrast_byte,
                mask,
                global.display_format,
                "contrast",
            )?
        {
            return Ok(());
        }

        if args.format == TextureFormat::Tiff
            && utils::is_tiff(&args.input)?
            && utils::stream_tiff_pointwise_masked(
                &args.input,
                &args.output,
                false,
                contrast_byte,
                mask,
                global.display_format,
                "contrast",
            )?
        {
            return Ok(());
        }

        return utils::buffered_fallback_masked(
            &args.input,
            &args.output,
            args.format.into(),
            global.no_memory_limit,
            false,
            contrast_byte,
            mask,
            global.display_format,
            "contrast",
        );
    }

    if args.format == TextureFormat::Png
        && utils::is_png(&args.input)?
        && utils::stream_png_pointwise(
            &args.input,
            &args.output,
            false,
            contrast_byte,
            global.display_format,
            "contrast",
        )?
    {
        return Ok(());
    }

    if args.format == TextureFormat::Tiff
        && utils::is_tiff(&args.input)?
        && utils::stream_tiff_pointwise(
            &args.input,
            &args.output,
            false,
            contrast_byte,
            global.display_format,
            "contrast",
        )?
    {
        return Ok(());
    }

    let _progress = Progress::new(global.display_format, "contrast", None);
    utils::buffered_fallback(
        &args.input,
        &args.output,
        args.format.into(),
        global.no_memory_limit,
        |img| image::imageops::colorops::contrast_in_place(img, args.contrast),
    )
}
