use clap::Parser;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::utils::{self, MaskArgs, Progress};
use crate::{ProcessError, TextureFormat};

#[derive(Parser, Clone)]
pub struct Brightness {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(short, long, allow_hyphen_values = true)]
    pub brightness: i32,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    #[clap(flatten)]
    pub mask: MaskArgs,
}

pub fn brightness(global: Cli, args: Brightness) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    // Matches `image::imageops::brighten`, which clamps `sample + value` into
    // 0..=255 and, like `invert`, leaves the alpha channel untouched.
    let brighten_byte = move |byte: u8| (i32::from(byte) + args.brightness).clamp(0, 255) as u8;

    if let Some(mask) = args.mask.resolve()? {
        if args.format == TextureFormat::Png
            && utils::is_png(&args.input)?
            && utils::stream_png_pointwise_masked(
                &args.input,
                &args.output,
                true,
                brighten_byte,
                mask,
                global.display_format,
                "brighten",
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
                brighten_byte,
                mask,
                global.display_format,
                "brighten",
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
            brighten_byte,
            mask,
            global.display_format,
            "brighten",
        );
    }

    if args.format == TextureFormat::Png
        && utils::is_png(&args.input)?
        && utils::stream_png_pointwise(
            &args.input,
            &args.output,
            true,
            brighten_byte,
            global.display_format,
            "brighten",
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
            brighten_byte,
            global.display_format,
            "brighten",
        )?
    {
        return Ok(());
    }

    let _progress = Progress::new(global.display_format, "brighten", None);
    utils::buffered_fallback(
        &args.input,
        &args.output,
        args.format.into(),
        global.no_memory_limit,
        |img| image::imageops::colorops::brighten_in_place(img, args.brightness),
    )
}
