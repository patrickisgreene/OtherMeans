use std::path::Path;

use image::GenericImageView;

use crate::cli::Cli;
use crate::format::DisplayFormat;
use crate::utils::{self, MaskSpec, Progress, RowSource, mask_byte_valid, mask_row_to_valid};
use crate::{ProcessError, TextureFormat};

/// Runs a binary (two-image), per-byte pointwise operation end to end:
/// the existing-output check, streaming PNG/TIFF when possible, and the
/// buffered fallback otherwise. `op_name` is used both for progress
/// reporting and in error messages; `combine` is applied independently to
/// every byte (every channel, alpha included) of the two inputs.
pub fn run_binary_op(
    global: &Cli,
    file1: &Path,
    file2: &Path,
    output: &Path,
    format: TextureFormat,
    op_name: &'static str,
    combine: impl Fn(u8, u8) -> u8,
) -> Result<(), ProcessError> {
    if output.exists() && !global.overwrite {
        println!("Output file `{}` already exists!", output.to_string_lossy());
        return Ok(());
    }

    match open_streamable_pair(file1, file2, format, op_name)? {
        Some((a, b)) => stream_binary_op(
            a,
            b,
            output,
            format,
            global.display_format,
            op_name,
            combine,
        ),
        None => buffered_binary_op(
            file1,
            file2,
            output,
            format,
            global.no_memory_limit,
            global.display_format,
            op_name,
            combine,
        ),
    }
}

type RowSourcePair = (Box<dyn RowSource>, Box<dyn RowSource>);

/// Opens both inputs as `RowSource`s for the streaming path. Returns
/// `Ok(None)` (not an error) if either isn't PNG/TIFF-streamable, if their
/// color layouts don't match (byte-for-byte combination needs identical
/// channel layouts on both sides), or if the combined layout has no TIFF
/// equivalent (the `tiff` crate's typed encoder has no grayscale+alpha
/// color type) — the caller should fall back to `buffered_binary_op` for
/// the whole operation in that case, which normalizes both to RGBA8 first.
pub fn open_streamable_pair(
    file1: &Path,
    file2: &Path,
    format: TextureFormat,
    op_name: &str,
) -> Result<Option<RowSourcePair>, ProcessError> {
    let (Some(a), Some(b)) = (
        utils::open_row_source(file1)?,
        utils::open_row_source(file2)?,
    ) else {
        return Ok(None);
    };

    if (a.width(), a.height()) != (b.width(), b.height()) {
        return Err(ProcessError::InvalidInput(format!(
            "`{}` is {}x{}, but `{}` is {}x{} — both images must be the same size to {op_name}",
            file1.display(),
            a.width(),
            a.height(),
            file2.display(),
            b.width(),
            b.height(),
        )));
    }

    let same_layout = a.channels() == b.channels()
        && a.is_gray() == b.is_gray()
        && a.has_alpha() == b.has_alpha();
    let tiff_representable = format != TextureFormat::Tiff || a.channels() != 2;

    if !same_layout || !tiff_representable {
        return Ok(None);
    }

    Ok(Some((a, b)))
}

pub fn stream_binary_op(
    mut a: Box<dyn RowSource>,
    mut b: Box<dyn RowSource>,
    output: &Path,
    format: TextureFormat,
    display_format: DisplayFormat,
    op_name: &'static str,
    combine: impl Fn(u8, u8) -> u8,
) -> Result<(), ProcessError> {
    let (width, height) = (a.width(), a.height());
    let (channels, is_gray, has_alpha) = (a.channels(), a.is_gray(), a.has_alpha());

    let mut next_row = move || -> Result<Vec<u8>, ProcessError> {
        let row_a = a.next_row()?.ok_or_else(|| {
            ProcessError::InvalidInput("the first image ended before the second".to_string())
        })?;
        let row_b = b.next_row()?.ok_or_else(|| {
            ProcessError::InvalidInput("the second image ended before the first".to_string())
        })?;
        Ok(row_a
            .iter()
            .zip(row_b.iter())
            .map(|(x, y)| combine(*x, *y))
            .collect())
    };

    let progress = Progress::new(display_format, op_name, Some(u64::from(height)));

    utils::write_rows_by_layout(
        output,
        width,
        height,
        format,
        channels,
        is_gray,
        has_alpha,
        &mut next_row,
        progress,
    )
}

/// Falls back to decoding both images fully via `image` — used when either
/// isn't PNG/TIFF, isn't in the row/strip-streamable shape those two
/// formats support, or the two sources have different color layouts.
/// Normalizes both to RGBA8 (so e.g. a grayscale source gets expanded to
/// R=G=B=luma, A=255 before combining) rather than rejecting mismatched
/// layouts outright, since there's no ambiguity in the combine once both
/// sides share a common 4-channel representation.
#[allow(clippy::too_many_arguments)]
pub fn buffered_binary_op(
    file1: &Path,
    file2: &Path,
    output: &Path,
    format: TextureFormat,
    no_memory_limit: bool,
    display_format: DisplayFormat,
    op_name: &'static str,
    combine: impl Fn(u8, u8) -> u8,
) -> Result<(), ProcessError> {
    let decode = |path: &Path| -> Result<image::DynamicImage, ProcessError> {
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        if no_memory_limit {
            reader.no_limits();
        }
        Ok(reader.decode()?)
    };

    let img1 = decode(file1)?;
    let img2 = decode(file2)?;

    if img1.dimensions() != img2.dimensions() {
        let (w1, h1) = img1.dimensions();
        let (w2, h2) = img2.dimensions();
        return Err(ProcessError::InvalidInput(format!(
            "`{}` is {w1}x{h1}, but `{}` is {w2}x{h2} — both images must be the same size to {op_name}",
            file1.display(),
            file2.display(),
        )));
    }

    let (width, height) = img1.dimensions();
    let rgba1 = img1.to_rgba8();
    let rgba2 = img2.to_rgba8();

    let mut progress = Progress::new(display_format, op_name, Some(u64::from(height)));
    let mut output_image = image::RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let p1 = rgba1.get_pixel(x, y).0;
            let p2 = rgba2.get_pixel(x, y).0;
            let mut result = [0u8; 4];
            for i in 0..4 {
                result[i] = combine(p1[i], p2[i]);
            }
            *output_image.get_pixel_mut(x, y) = image::Rgba(result);
        }
        progress.inc(1);
    }

    output_image.save_with_format(output, format.into())?;
    Ok(())
}

/// Masked entry point, mirroring `run_binary_op`. Masked pixels pass
/// through `file1`'s value unchanged (the "primary" image — e.g. for
/// `sub`, that's the minuend), not some blend of the two.
#[allow(clippy::too_many_arguments)]
pub fn run_binary_op_masked(
    global: &Cli,
    file1: &Path,
    file2: &Path,
    mask: MaskSpec,
    output: &Path,
    format: TextureFormat,
    op_name: &'static str,
    combine: impl Fn(u8, u8) -> u8,
) -> Result<(), ProcessError> {
    if output.exists() && !global.overwrite {
        println!("Output file `{}` already exists!", output.to_string_lossy());
        return Ok(());
    }

    match open_streamable_masked_triple(file1, file2, mask.path, format, op_name)? {
        Some((a, b, m)) => stream_binary_op_masked(
            a,
            b,
            m,
            mask.excludes_white,
            output,
            format,
            global.display_format,
            op_name,
            combine,
        ),
        None => buffered_binary_op_masked(
            file1,
            file2,
            mask,
            output,
            format,
            global.no_memory_limit,
            global.display_format,
            op_name,
            combine,
        ),
    }
}

type RowSourceTriple = (Box<dyn RowSource>, Box<dyn RowSource>, Box<dyn RowSource>);

/// Opens all three inputs as `RowSource`s for the streaming masked path.
/// Reuses `open_streamable_pair` for the existing file1/file2 checks (same
/// dimensions, same channel layout, TIFF-representable), then additionally
/// opens the mask and requires it to match file1/file2's *dimensions* only
/// — not their channel layout, since the mask is only ever read via its
/// channel-0 byte (see `mask_row_to_valid`).
pub fn open_streamable_masked_triple(
    file1: &Path,
    file2: &Path,
    mask: &Path,
    format: TextureFormat,
    op_name: &str,
) -> Result<Option<RowSourceTriple>, ProcessError> {
    let Some((a, b)) = open_streamable_pair(file1, file2, format, op_name)? else {
        return Ok(None);
    };
    let Some(m) = utils::open_row_source(mask)? else {
        return Ok(None);
    };

    if (a.width(), a.height()) != (m.width(), m.height()) {
        return Err(ProcessError::InvalidInput(format!(
            "mask `{}` is {}x{}, but the images being combined are {}x{} — the mask must match their dimensions",
            mask.display(),
            m.width(),
            m.height(),
            a.width(),
            a.height(),
        )));
    }

    Ok(Some((a, b, m)))
}

/// Masked variant of `stream_binary_op`: pulls a mask row each iteration
/// alongside the two image rows, and gates `combine` vs. a passthrough of
/// `a`'s byte per pixel index instead of always combining.
#[allow(clippy::too_many_arguments)]
pub fn stream_binary_op_masked(
    mut a: Box<dyn RowSource>,
    mut b: Box<dyn RowSource>,
    mut mask: Box<dyn RowSource>,
    mask_excludes_white: bool,
    output: &Path,
    format: TextureFormat,
    display_format: DisplayFormat,
    op_name: &'static str,
    combine: impl Fn(u8, u8) -> u8,
) -> Result<(), ProcessError> {
    let (width, height) = (a.width(), a.height());
    let (channels, is_gray, has_alpha) = (a.channels(), a.is_gray(), a.has_alpha());
    let mask_channels = mask.channels();

    let mut next_row = move || -> Result<Vec<u8>, ProcessError> {
        let row_a = a.next_row()?.ok_or_else(|| {
            ProcessError::InvalidInput("the first image ended before the second".to_string())
        })?;
        let row_b = b.next_row()?.ok_or_else(|| {
            ProcessError::InvalidInput("the second image ended before the first".to_string())
        })?;
        let mask_row = mask.next_row()?.ok_or_else(|| {
            ProcessError::InvalidInput("the mask ended before the images".to_string())
        })?;
        let valid = mask_row_to_valid(&mask_row, mask_channels, mask_excludes_white);

        Ok(row_a
            .iter()
            .zip(row_b.iter())
            .enumerate()
            .map(|(i, (x, y))| {
                if valid[i / channels] {
                    combine(*x, *y)
                } else {
                    *x
                }
            })
            .collect())
    };

    let progress = Progress::new(display_format, op_name, Some(u64::from(height)));

    utils::write_rows_by_layout(
        output,
        width,
        height,
        format,
        channels,
        is_gray,
        has_alpha,
        &mut next_row,
        progress,
    )
}

/// Masked variant of `buffered_binary_op`: decodes all three (file1/file2/
/// mask) to RGBA8, gates the per-channel `combine` loop by the mask's
/// validity per pixel, passing through `p1` unchanged when masked.
#[allow(clippy::too_many_arguments)]
pub fn buffered_binary_op_masked(
    file1: &Path,
    file2: &Path,
    mask: MaskSpec,
    output: &Path,
    format: TextureFormat,
    no_memory_limit: bool,
    display_format: DisplayFormat,
    op_name: &'static str,
    combine: impl Fn(u8, u8) -> u8,
) -> Result<(), ProcessError> {
    let decode = |path: &Path| -> Result<image::DynamicImage, ProcessError> {
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        if no_memory_limit {
            reader.no_limits();
        }
        Ok(reader.decode()?)
    };

    let img1 = decode(file1)?;
    let img2 = decode(file2)?;
    let mask_img = decode(mask.path)?;

    if img1.dimensions() != img2.dimensions() {
        let (w1, h1) = img1.dimensions();
        let (w2, h2) = img2.dimensions();
        return Err(ProcessError::InvalidInput(format!(
            "`{}` is {w1}x{h1}, but `{}` is {w2}x{h2} — both images must be the same size to {op_name}",
            file1.display(),
            file2.display(),
        )));
    }

    let (width, height) = img1.dimensions();
    if mask_img.dimensions() != (width, height) {
        let (mw, mh) = mask_img.dimensions();
        return Err(ProcessError::InvalidInput(format!(
            "mask `{}` is {mw}x{mh}, but the images being combined are {width}x{height} — the mask must match their dimensions",
            mask.path.display(),
        )));
    }

    let rgba1 = img1.to_rgba8();
    let rgba2 = img2.to_rgba8();
    let mask_rgba = mask_img.to_rgba8();

    let mut progress = Progress::new(display_format, op_name, Some(u64::from(height)));
    let mut output_image = image::RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let p1 = rgba1.get_pixel(x, y).0;
            let p2 = rgba2.get_pixel(x, y).0;
            let mut result = p1;
            if mask_byte_valid(mask_rgba.get_pixel(x, y).0[0], mask.excludes_white) {
                for i in 0..4 {
                    result[i] = combine(p1[i], p2[i]);
                }
            }
            *output_image.get_pixel_mut(x, y) = image::Rgba(result);
        }
        progress.inc(1);
    }

    output_image.save_with_format(output, format.into())?;
    Ok(())
}
