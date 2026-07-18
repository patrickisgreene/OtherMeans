use std::path::Path;

use image::GenericImageView;

use super::*;
use crate::{DisplayFormat, ProcessError, TextureFormat};

pub fn role_index(role: Role) -> usize {
    match role {
        Role::Red => 0,
        Role::Green => 1,
        Role::Blue => 2,
        Role::Alpha => 3,
    }
}

pub type RoleSources = Vec<(Role, Box<dyn RowSource>)>;

/// Opens every role's source as a `RowSource`, for the streaming path.
/// Returns `Ok(None)` (not an error) the moment any source isn't PNG/TIFF
/// or doesn't fit the row/strip-streamable shape those formats support —
/// the caller should fall back to `buffered_concat` for the whole
/// operation in that case, rather than mixing streamed and buffered
/// sources.
pub fn open_all_row_sources(roles: &[(Role, &Path)]) -> Result<Option<RoleSources>, ProcessError> {
    let mut sources = Vec::with_capacity(roles.len());
    for &(role, path) in roles {
        match open_row_source(path)? {
            Some(source) => sources.push((role, source)),
            None => return Ok(None),
        }
    }

    let (width, height) = (sources[0].1.width(), sources[0].1.height());
    for (role, source) in &sources {
        if (source.width(), source.height()) != (width, height) {
            return Err(ProcessError::InvalidInput(format!(
                "`--{}` source is {}x{}, but expected {width}x{height} to match the other channel sources",
                role.flag_name(),
                source.width(),
                source.height(),
            )));
        }
    }

    Ok(Some(sources))
}

pub fn stream_concat(
    mut sources: RoleSources,
    out_channels: usize,
    output: &Path,
    format: TextureFormat,
    display_format: DisplayFormat,
) -> Result<(), ProcessError> {
    let width = sources[0].1.width();
    let height = sources[0].1.height();

    let mut next_row = move || -> Result<Vec<u8>, ProcessError> {
        let mut out_row = vec![0u8; width as usize * out_channels];
        for (role, source) in sources.iter_mut() {
            let row = source.next_row()?.ok_or_else(|| {
                ProcessError::InvalidInput(format!(
                    "`--{}` source ended before the others (all channel sources must be the same size)",
                    role.flag_name()
                ))
            })?;
            let values = extract_role(source.as_ref(), &row, *role)?;
            let index = role_index(*role);
            for (x, value) in values.into_iter().enumerate() {
                out_row[x * out_channels + index] = value;
            }
        }
        Ok(out_row)
    };

    let progress = Progress::new(display_format, "rgba-concat", Some(u64::from(height)));

    match format {
        TextureFormat::Png => {
            let color_type = if out_channels == 4 {
                png::ColorType::Rgba
            } else {
                png::ColorType::Rgb
            };
            write_png_rows(output, width, height, color_type, &mut next_row, progress)
        }
        TextureFormat::Tiff if out_channels == 4 => {
            write_tiff_rows::<tiff::encoder::colortype::RGBA8>(
                output,
                width,
                height,
                &mut next_row,
                progress,
            )
        }
        TextureFormat::Tiff => write_tiff_rows::<tiff::encoder::colortype::RGB8>(
            output,
            width,
            height,
            &mut next_row,
            progress,
        ),
    }
}

/// Falls back to decoding every source fully via `image` — used when any
/// source isn't PNG/TIFF, or isn't in the row/strip-streamable shape those
/// two formats support (interlacing, indexed color, tiled/planar TIFFs,
/// depth over 8 bits per sample). Mirrors `extract_role`'s channel-picking
/// semantics on `image::Rgba` pixels instead of raw decoded row bytes.
pub fn buffered_concat(
    roles: &[(Role, &Path)],
    out_channels: usize,
    output: &Path,
    format: TextureFormat,
    no_memory_limit: bool,
    display_format: DisplayFormat,
) -> Result<(), ProcessError> {
    let mut images = Vec::with_capacity(roles.len());
    for &(role, path) in roles {
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        if no_memory_limit {
            reader.no_limits();
        }
        images.push((role, reader.decode()?));
    }

    let (width, height) = images[0].1.dimensions();
    for (role, img) in &images {
        if img.dimensions() != (width, height) {
            let (w, h) = img.dimensions();
            return Err(ProcessError::InvalidInput(format!(
                "`--{}` source is {w}x{h}, but expected {width}x{height} to match the other channel sources",
                role.flag_name(),
            )));
        }
    }

    // (output channel index, this source's RGBA8 buffer, channel index to read from it)
    let mut source_channels = Vec::with_capacity(images.len());
    for (role, img) in &images {
        let color = img.color();
        let is_gray = matches!(
            color,
            image::ColorType::L8
                | image::ColorType::La8
                | image::ColorType::L16
                | image::ColorType::La16
        );
        let has_alpha = color.has_alpha();

        let channel = match (is_gray, *role) {
            (true, Role::Alpha) if has_alpha => 3,
            (true, _) => 0,
            (false, Role::Red) => 0,
            (false, Role::Green) => 1,
            (false, Role::Blue) => 2,
            (false, Role::Alpha) if has_alpha => 3,
            (false, Role::Alpha) => {
                return Err(ProcessError::InvalidInput(
                    "`--alpha` source has no alpha channel to extract (pass an RGBA/grayscale+alpha \
                     image, or omit `--alpha` for RGB output)"
                        .to_string(),
                ));
            }
        };

        source_channels.push((role_index(*role), img.to_rgba8(), channel));
    }
    drop(images);

    let mut progress = Progress::new(display_format, "rgba-concat", Some(u64::from(height)));
    let mut output_image = image::RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let out_pixel = output_image.get_pixel_mut(x, y);
            for (out_index, rgba, channel) in &source_channels {
                out_pixel.0[*out_index] = rgba.get_pixel(x, y).0[*channel];
            }
        }
        progress.inc(1);
    }

    if out_channels == 4 {
        output_image.save_with_format(output, format.into())?;
    } else {
        image::DynamicImage::ImageRgba8(output_image)
            .to_rgb8()
            .save_with_format(output, format.into())?;
    }

    Ok(())
}
