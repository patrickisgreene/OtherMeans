use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use super::Progress;
use crate::format::DisplayFormat;
use crate::utils::{self, MaskSpec, RowSource, mask_byte_valid, mask_row_to_valid};
use crate::{ProcessError, TextureFormat};

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub fn is_png(path: &Path) -> Result<bool, ProcessError> {
    let mut signature = [0u8; 8];
    Ok(File::open(path)?.read_exact(&mut signature).is_ok() && signature == PNG_SIGNATURE)
}

pub fn is_tiff(path: &Path) -> Result<bool, ProcessError> {
    let mut signature = [0u8; 4];
    if !File::open(path)?.read_exact(&mut signature).is_ok() {
        return Ok(false);
    }
    Ok(signature == [0x49, 0x49, 0x2A, 0x00] || signature == [0x4D, 0x4D, 0x00, 0x2A])
}

/// Decodes, applies `apply` in place, and re-encodes. Used both as the
/// top-level fallback (output format that isn't PNG/TIFF, or an input that
/// doesn't match the requested output format) and as the fallback for
/// PNG/TIFF layouts `stream_png_pointwise`/`stream_tiff_pointwise` don't
/// support (interlacing, indexed color, tiled/planar TIFFs, >8-bit depth).
///
/// `image`'s decoders reject anything needing more than 512 MiB by default
/// (`LimitErrorKind::InsufficientMemory`) as a guard against decompression
/// bombs. `no_memory_limit` lifts that guard for callers that know their
/// inputs are large but legitimate (e.g. big GeoTIFFs).
pub fn buffered_fallback(
    input: &Path,
    output: &Path,
    format: image::ImageFormat,
    no_memory_limit: bool,
    apply: impl FnOnce(&mut image::DynamicImage),
) -> Result<(), ProcessError> {
    let mut reader = image::ImageReader::open(input)?.with_guessed_format()?;
    if no_memory_limit {
        reader.no_limits();
    }
    let mut input_image = reader.decode()?;
    apply(&mut input_image);
    Ok(input_image.write_to(&mut File::create(output)?, format)?)
}

/// Masked variant of `buffered_fallback`. A per-pixel mask gate doesn't fit
/// `image::imageops`'s "transform the whole image" functions, so instead of
/// `apply: FnOnce(&mut DynamicImage)` this takes the same per-byte
/// `transform`/`alpha_exempt` the streaming path uses (already proven
/// byte-identical to `image::imageops`'s exact math for invert/brighten/
/// contrast), applied manually per pixel after normalizing both image and
/// mask to RGBA8 — the same simplification `buffered_binary_op`/
/// `buffered_masked_blur` already make. Unlike `buffered_fallback` (one
/// opaque blocking call, no progress reporting), this loops row by row, so
/// it reports real progress like `buffered_binary_op` does.
#[allow(clippy::too_many_arguments)]
pub fn buffered_fallback_masked(
    input: &Path,
    output: &Path,
    format: image::ImageFormat,
    no_memory_limit: bool,
    alpha_exempt: bool,
    transform: impl Fn(u8) -> u8,
    mask: MaskSpec,
    display_format: DisplayFormat,
    op: &'static str,
) -> Result<(), ProcessError> {
    let decode = |path: &Path| -> Result<image::DynamicImage, ProcessError> {
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        if no_memory_limit {
            reader.no_limits();
        }
        Ok(reader.decode()?)
    };

    let img = decode(input)?.to_rgba8();
    let mask_img = decode(mask.path)?.to_rgba8();
    let (width, height) = (img.width(), img.height());

    if (mask_img.width(), mask_img.height()) != (width, height) {
        return Err(ProcessError::InvalidInput(format!(
            "mask `{}` is {}x{}, but image `{}` is {}x{} — the mask must match the image's dimensions",
            mask.path.display(),
            mask_img.width(),
            mask_img.height(),
            input.display(),
            width,
            height,
        )));
    }

    let mut progress = Progress::new(display_format, op, Some(u64::from(height)));
    let mut out = image::RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let mut px = img.get_pixel(x, y).0;
            if mask_byte_valid(mask_img.get_pixel(x, y).0[0], mask.excludes_white) {
                for (c, byte) in px.iter_mut().enumerate() {
                    if !(alpha_exempt && c == 3) {
                        *byte = transform(*byte);
                    }
                }
            }
            *out.get_pixel_mut(x, y) = image::Rgba(px);
        }
        progress.inc(1);
    }
    out.save_with_format(output, format)?;
    Ok(())
}

/// Applies a per-sample `transform` to a PNG one row at a time instead of
/// decoding the whole pixel buffer up front: each row is unfiltered by
/// `next_row`, transformed, and handed straight to `StreamWriter`, so peak
/// memory stays proportional to one row rather than to the full image.
///
/// When `alpha_exempt` is set, the last channel of each pixel (the alpha
/// channel, for color types that have one) is left untouched rather than
/// passed through `transform` — this matches `image::imageops`, where
/// `invert`/`brighten_in_place` skip alpha (via `map_with_alpha`) but
/// `contrast_in_place` does not (it uses a plain `map` over every channel).
///
/// Returns `Ok(false)` (rather than falling back itself) for anything
/// `next_row` can't hand back as flat, directly transformable samples:
/// interlaced PNGs (rows arrive out of raster order), indexed color
/// (samples are palette indices, not pixel values), and >8-bit depth
/// (samples span two bytes). Callers should fall back to `buffered_fallback`.
pub fn stream_png_pointwise(
    input: &Path,
    output: &Path,
    alpha_exempt: bool,
    transform: impl Fn(u8) -> u8,
    display_format: DisplayFormat,
    op: &'static str,
) -> Result<bool, ProcessError> {
    let decoder = png::Decoder::new(BufReader::new(File::open(input)?));
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let (width, height, color_type, bit_depth, interlaced) = (
        info.width,
        info.height,
        info.color_type,
        info.bit_depth,
        info.interlaced,
    );

    if interlaced || bit_depth != png::BitDepth::Eight || color_type == png::ColorType::Indexed {
        return Ok(false);
    }

    let channels = color_type.samples();
    let has_alpha = alpha_exempt
        && matches!(
            color_type,
            png::ColorType::GrayscaleAlpha | png::ColorType::Rgba
        );

    let mut encoder = png::Encoder::new(BufWriter::new(File::create(output)?), width, height);
    encoder.set_color(color_type);
    encoder.set_depth(bit_depth);
    let mut stream_writer = encoder.write_header()?.into_stream_writer()?;

    let mut progress = Progress::new(display_format, op, Some(u64::from(height)));
    let mut row_buf = Vec::new();
    while let Some(row) = reader.next_row()? {
        row_buf.clear();
        row_buf.extend(row.data().iter().enumerate().map(|(i, &byte)| {
            if has_alpha && i % channels == channels - 1 {
                byte
            } else {
                transform(byte)
            }
        }));
        stream_writer.write_all(&row_buf)?;
        progress.inc(1);
    }

    stream_writer.finish()?;
    Ok(true)
}

/// Masked variant of `stream_png_pointwise`: an extra black/white mask
/// image is read row-by-row in lockstep (via `open_row_source`, so it can
/// be PNG or TIFF independent of `input`'s own format), and pixels it
/// marks excluded are left untouched in the output rather than having
/// `transform` applied — matching `alpha_exempt`'s per-channel skip, but
/// gating the *whole pixel* instead of just one channel.
///
/// Returns `Ok(false)` (not falling back itself) for the same reasons
/// `stream_png_pointwise` does, plus if the mask itself isn't PNG/TIFF-
/// streamable. A mask/image dimension mismatch is a hard error either way,
/// since there's no sensible way to align them.
#[allow(clippy::too_many_arguments)]
pub fn stream_png_pointwise_masked(
    input: &Path,
    output: &Path,
    alpha_exempt: bool,
    transform: impl Fn(u8) -> u8,
    mask: MaskSpec,
    display_format: DisplayFormat,
    op: &'static str,
) -> Result<bool, ProcessError> {
    let Some(mut mask_source) = utils::open_row_source(mask.path)? else {
        return Ok(false);
    };

    let decoder = png::Decoder::new(BufReader::new(File::open(input)?));
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let (width, height, color_type, bit_depth, interlaced) = (
        info.width,
        info.height,
        info.color_type,
        info.bit_depth,
        info.interlaced,
    );

    if interlaced || bit_depth != png::BitDepth::Eight || color_type == png::ColorType::Indexed {
        return Ok(false);
    }

    if (mask_source.width(), mask_source.height()) != (width, height) {
        return Err(ProcessError::InvalidInput(format!(
            "mask `{}` is {}x{}, but image `{}` is {}x{} — the mask must match the image's dimensions",
            mask.path.display(),
            mask_source.width(),
            mask_source.height(),
            input.display(),
            width,
            height,
        )));
    }

    let channels = color_type.samples();
    let mask_channels = mask_source.channels();
    let has_alpha = alpha_exempt
        && matches!(
            color_type,
            png::ColorType::GrayscaleAlpha | png::ColorType::Rgba
        );

    let mut encoder = png::Encoder::new(BufWriter::new(File::create(output)?), width, height);
    encoder.set_color(color_type);
    encoder.set_depth(bit_depth);
    let mut stream_writer = encoder.write_header()?.into_stream_writer()?;

    let mut progress = Progress::new(display_format, op, Some(u64::from(height)));
    let mut row_buf = Vec::new();
    while let Some(row) = reader.next_row()? {
        let mask_row = mask_source.next_row()?.ok_or_else(|| {
            ProcessError::InvalidInput("mask source ended before image".to_string())
        })?;
        let valid = mask_row_to_valid(&mask_row, mask_channels, mask.excludes_white);

        row_buf.clear();
        row_buf.extend(row.data().iter().enumerate().map(|(i, &byte)| {
            let masked = !valid[i / channels];
            let alpha_channel = has_alpha && i % channels == channels - 1;
            if masked || alpha_channel {
                byte
            } else {
                transform(byte)
            }
        }));
        stream_writer.write_all(&row_buf)?;
        progress.inc(1);
    }

    stream_writer.finish()?;
    Ok(true)
}

/// Applies a per-sample `transform` to a TIFF strip by strip: TIFF strips
/// are independently compressed and directly addressable via
/// `read_chunk`/`write_strip` (no shared deflate state to carry across them
/// like PNG's row stream), so peak memory stays proportional to one strip
/// rather than to the full image. See `stream_png_pointwise` for what
/// `alpha_exempt` means.
///
/// Returns `Ok(false)` (rather than falling back itself) for anything that
/// doesn't fit the simple one-strip-in, one-strip-out mapping this uses:
/// tiled (rather than stripped) layouts, planar (non-interleaved) channel
/// storage, and anything other than 8-bit Grayscale/RGB/RGBA samples.
/// Callers should fall back to `buffered_fallback`.
pub fn stream_tiff_pointwise(
    input: &Path,
    output: &Path,
    alpha_exempt: bool,
    transform: impl Fn(u8) -> u8 + Copy,
    display_format: DisplayFormat,
    op: &'static str,
) -> Result<bool, ProcessError> {
    let mut decoder = tiff::decoder::Decoder::new(BufReader::new(File::open(input)?))?;

    let is_chunky = decoder
        .find_tag_unsigned::<u16>(tiff::tags::Tag::PlanarConfiguration)?
        .unwrap_or(1)
        == 1;

    if decoder.get_chunk_type() != tiff::decoder::ChunkType::Strip || !is_chunky {
        return Ok(false);
    }

    let (width, height) = decoder.dimensions()?;
    let color_type = decoder.colortype()?;
    if !matches!(
        color_type,
        tiff::ColorType::Gray(8) | tiff::ColorType::RGB(8) | tiff::ColorType::RGBA(8)
    ) {
        return Ok(false);
    }

    let strip_count = decoder.strip_count()?;
    let rows_per_strip = decoder
        .find_tag_unsigned::<u32>(tiff::tags::Tag::RowsPerStrip)?
        .unwrap_or(height);

    let mut tiff_encoder = tiff::encoder::TiffEncoder::new(BufWriter::new(File::create(output)?))?;

    let layout = TiffStripLayout {
        width,
        height,
        strip_count,
        rows_per_strip,
    };

    let has_alpha = alpha_exempt && matches!(color_type, tiff::ColorType::RGBA(8));
    let mut progress = Progress::new(display_format, op, Some(u64::from(strip_count)));

    match color_type {
        tiff::ColorType::Gray(8) => stream_tiff_strips::<tiff::encoder::colortype::Gray8>(
            &mut decoder,
            &mut tiff_encoder,
            layout,
            1,
            has_alpha,
            transform,
            &mut progress,
        )?,
        tiff::ColorType::RGB(8) => stream_tiff_strips::<tiff::encoder::colortype::RGB8>(
            &mut decoder,
            &mut tiff_encoder,
            layout,
            3,
            has_alpha,
            transform,
            &mut progress,
        )?,
        tiff::ColorType::RGBA(8) => stream_tiff_strips::<tiff::encoder::colortype::RGBA8>(
            &mut decoder,
            &mut tiff_encoder,
            layout,
            4,
            has_alpha,
            transform,
            &mut progress,
        )?,
        _ => unreachable!("checked above"),
    }

    Ok(true)
}

/// Masked variant of `stream_tiff_pointwise` — see `stream_png_pointwise_masked`
/// for the general shape. The mask is row-granular while TIFF strips span
/// multiple rows, so `stream_tiff_strips_masked` pulls one mask row per row
/// within each strip rather than one per strip.
#[allow(clippy::too_many_arguments)]
pub fn stream_tiff_pointwise_masked(
    input: &Path,
    output: &Path,
    alpha_exempt: bool,
    transform: impl Fn(u8) -> u8 + Copy,
    mask: MaskSpec,
    display_format: DisplayFormat,
    op: &'static str,
) -> Result<bool, ProcessError> {
    let Some(mut mask_source) = utils::open_row_source(mask.path)? else {
        return Ok(false);
    };

    let mut decoder = tiff::decoder::Decoder::new(BufReader::new(File::open(input)?))?;

    let is_chunky = decoder
        .find_tag_unsigned::<u16>(tiff::tags::Tag::PlanarConfiguration)?
        .unwrap_or(1)
        == 1;

    if decoder.get_chunk_type() != tiff::decoder::ChunkType::Strip || !is_chunky {
        return Ok(false);
    }

    let (width, height) = decoder.dimensions()?;
    let color_type = decoder.colortype()?;
    if !matches!(
        color_type,
        tiff::ColorType::Gray(8) | tiff::ColorType::RGB(8) | tiff::ColorType::RGBA(8)
    ) {
        return Ok(false);
    }

    if (mask_source.width(), mask_source.height()) != (width, height) {
        return Err(ProcessError::InvalidInput(format!(
            "mask `{}` is {}x{}, but image `{}` is {}x{} — the mask must match the image's dimensions",
            mask.path.display(),
            mask_source.width(),
            mask_source.height(),
            input.display(),
            width,
            height,
        )));
    }

    let strip_count = decoder.strip_count()?;
    let rows_per_strip = decoder
        .find_tag_unsigned::<u32>(tiff::tags::Tag::RowsPerStrip)?
        .unwrap_or(height);

    let mut tiff_encoder = tiff::encoder::TiffEncoder::new(BufWriter::new(File::create(output)?))?;

    let layout = TiffStripLayout {
        width,
        height,
        strip_count,
        rows_per_strip,
    };

    let has_alpha = alpha_exempt && matches!(color_type, tiff::ColorType::RGBA(8));
    let mask_channels = mask_source.channels();
    let mut progress = Progress::new(display_format, op, Some(u64::from(strip_count)));

    match color_type {
        tiff::ColorType::Gray(8) => stream_tiff_strips_masked::<tiff::encoder::colortype::Gray8>(
            &mut decoder,
            &mut tiff_encoder,
            layout,
            1,
            has_alpha,
            transform,
            &mut *mask_source,
            mask_channels,
            mask.excludes_white,
            &mut progress,
        )?,
        tiff::ColorType::RGB(8) => stream_tiff_strips_masked::<tiff::encoder::colortype::RGB8>(
            &mut decoder,
            &mut tiff_encoder,
            layout,
            3,
            has_alpha,
            transform,
            &mut *mask_source,
            mask_channels,
            mask.excludes_white,
            &mut progress,
        )?,
        tiff::ColorType::RGBA(8) => stream_tiff_strips_masked::<tiff::encoder::colortype::RGBA8>(
            &mut decoder,
            &mut tiff_encoder,
            layout,
            4,
            has_alpha,
            transform,
            &mut *mask_source,
            mask_channels,
            mask.excludes_white,
            &mut progress,
        )?,
        _ => unreachable!("checked above"),
    }

    Ok(true)
}

/// Writes `height` rows (each produced on demand by `next_row`) out as a
/// PNG, streaming them straight into a `StreamWriter` one at a time. Used
/// by multi-input row-combining ops (`add`, `rgba_concat`) whose output
/// rows are synthesized from more than one source rather than transformed
/// from a single one, so they can't reuse `stream_png_pointwise`.
pub fn write_png_rows(
    output: &Path,
    width: u32,
    height: u32,
    color_type: png::ColorType,
    next_row: &mut dyn FnMut() -> Result<Vec<u8>, ProcessError>,
    mut progress: Progress,
) -> Result<(), ProcessError> {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(output)?), width, height);
    encoder.set_color(color_type);
    encoder.set_depth(png::BitDepth::Eight);
    let mut stream_writer = encoder.write_header()?.into_stream_writer()?;

    for _ in 0..height {
        let row = next_row()?;
        stream_writer.write_all(&row)?;
        progress.inc(1);
    }

    stream_writer.finish()?;
    Ok(())
}

/// Writes `height` rows (each produced on demand by `next_row`) out as a
/// TIFF, buffering them into strip-sized chunks sized by the encoder's own
/// default `rows_per_strip` (there's no single input strip layout to
/// mirror here, unlike `stream_tiff_pointwise`). See `write_png_rows` for
/// why this exists alongside the single-input streaming functions.
pub fn write_tiff_rows<C>(
    output: &Path,
    width: u32,
    height: u32,
    next_row: &mut dyn FnMut() -> Result<Vec<u8>, ProcessError>,
    mut progress: Progress,
) -> Result<(), ProcessError>
where
    C: tiff::encoder::colortype::ColorType<Inner = u8>,
    [u8]: tiff::encoder::TiffValue,
{
    let mut tiff_encoder = tiff::encoder::TiffEncoder::new(BufWriter::new(File::create(output)?))?;
    let mut image_encoder = tiff_encoder.new_image::<C>(width, height)?;

    loop {
        let samples = image_encoder.next_strip_sample_count();
        if samples == 0 {
            break;
        }
        let mut strip = Vec::with_capacity(samples as usize);
        while (strip.len() as u64) < samples {
            strip.extend(next_row()?);
            progress.inc(1);
        }
        image_encoder.write_strip(&strip)?;
    }

    image_encoder.finish()?;
    Ok(())
}

fn png_color_type(channels: usize, is_gray: bool, has_alpha: bool) -> png::ColorType {
    match (channels, is_gray, has_alpha) {
        (1, true, false) => png::ColorType::Grayscale,
        (2, true, true) => png::ColorType::GrayscaleAlpha,
        (3, false, false) => png::ColorType::Rgb,
        (4, false, true) => png::ColorType::Rgba,
        _ => unreachable!("PngRowSource/TiffRowSource only produce these four layouts"),
    }
}

/// Dispatches to `write_png_rows` or the correctly-typed `write_tiff_rows::<C>`
/// based on a `RowSource`-style layout descriptor (`channels`/`is_gray`/
/// `has_alpha`). Shared by every op that synthesizes output rows on the fly
/// rather than transforming a single input byte-for-byte: `add`/`sub`/`mul`
/// (via `stream_binary_op`) and `blur` (via `stream_blur`).
#[allow(clippy::too_many_arguments)]
pub fn write_rows_by_layout(
    output: &Path,
    width: u32,
    height: u32,
    format: TextureFormat,
    channels: usize,
    is_gray: bool,
    has_alpha: bool,
    next_row: &mut dyn FnMut() -> Result<Vec<u8>, ProcessError>,
    progress: Progress,
) -> Result<(), ProcessError> {
    match format {
        TextureFormat::Png => {
            let color_type = png_color_type(channels, is_gray, has_alpha);
            write_png_rows(output, width, height, color_type, next_row, progress)
        }
        TextureFormat::Tiff => match (channels, is_gray, has_alpha) {
            (1, true, false) => write_tiff_rows::<tiff::encoder::colortype::Gray8>(
                output, width, height, next_row, progress,
            ),
            (3, false, false) => write_tiff_rows::<tiff::encoder::colortype::RGB8>(
                output, width, height, next_row, progress,
            ),
            (4, false, true) => write_tiff_rows::<tiff::encoder::colortype::RGBA8>(
                output, width, height, next_row, progress,
            ),
            _ => Err(ProcessError::InvalidInput(format!(
                "TIFF output has no color type for {channels}-channel data (is_gray={is_gray}, has_alpha={has_alpha})"
            ))),
        },
    }
}

#[derive(Clone, Copy)]
struct TiffStripLayout {
    width: u32,
    height: u32,
    strip_count: u32,
    rows_per_strip: u32,
}

fn stream_tiff_strips<C>(
    decoder: &mut tiff::decoder::Decoder<BufReader<File>>,
    encoder: &mut tiff::encoder::TiffEncoder<BufWriter<File>>,
    layout: TiffStripLayout,
    channels: usize,
    has_alpha: bool,
    transform: impl Fn(u8) -> u8,
    progress: &mut Progress,
) -> Result<(), ProcessError>
where
    C: tiff::encoder::colortype::ColorType<Inner = u8>,
    [u8]: tiff::encoder::TiffValue,
{
    let mut image_encoder = encoder.new_image::<C>(layout.width, layout.height)?;
    image_encoder.rows_per_strip(layout.rows_per_strip)?;

    for chunk_index in 0..layout.strip_count {
        let tiff::decoder::DecodingResult::U8(mut strip) = decoder.read_chunk(chunk_index)? else {
            unreachable!("decoder.colortype() reported an 8-bit sample type")
        };

        for (i, byte) in strip.iter_mut().enumerate() {
            if !(has_alpha && i % channels == channels - 1) {
                *byte = transform(*byte);
            }
        }

        image_encoder.write_strip(&strip)?;
        progress.inc(1);
    }

    image_encoder.finish()?;
    Ok(())
}

/// Masked variant of `stream_tiff_strips`: pulls one mask row per row
/// within each strip (a strip spans `rows_per_strip` rows; the mask is
/// row-granular, not strip-granular) and gates the existing per-byte
/// transform loop by pixel index instead of applying it unconditionally.
#[allow(clippy::too_many_arguments)]
fn stream_tiff_strips_masked<C>(
    decoder: &mut tiff::decoder::Decoder<BufReader<File>>,
    encoder: &mut tiff::encoder::TiffEncoder<BufWriter<File>>,
    layout: TiffStripLayout,
    channels: usize,
    has_alpha: bool,
    transform: impl Fn(u8) -> u8,
    mask_source: &mut dyn RowSource,
    mask_channels: usize,
    mask_excludes_white: bool,
    progress: &mut Progress,
) -> Result<(), ProcessError>
where
    C: tiff::encoder::colortype::ColorType<Inner = u8>,
    [u8]: tiff::encoder::TiffValue,
{
    let mut image_encoder = encoder.new_image::<C>(layout.width, layout.height)?;
    image_encoder.rows_per_strip(layout.rows_per_strip)?;

    let row_len = layout.width as usize * channels;

    for chunk_index in 0..layout.strip_count {
        let tiff::decoder::DecodingResult::U8(mut strip) = decoder.read_chunk(chunk_index)? else {
            unreachable!("decoder.colortype() reported an 8-bit sample type")
        };

        let rows_in_strip = strip.len() / row_len;
        for row_idx in 0..rows_in_strip {
            let mask_row = mask_source.next_row()?.ok_or_else(|| {
                ProcessError::InvalidInput("mask source ended before image".to_string())
            })?;
            let valid = mask_row_to_valid(&mask_row, mask_channels, mask_excludes_white);
            let row = &mut strip[row_idx * row_len..(row_idx + 1) * row_len];
            for (i, byte) in row.iter_mut().enumerate() {
                let pixel = i / channels;
                if valid[pixel] && !(has_alpha && i % channels == channels - 1) {
                    *byte = transform(*byte);
                }
            }
        }

        image_encoder.write_strip(&strip)?;
        progress.inc(1);
    }

    image_encoder.finish()?;
    Ok(())
}
