use std::cmp::min;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::ProcessError;

/// Which output channel a source image's data is being placed into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Red,
    Green,
    Blue,
    Alpha,
}

impl Role {
    pub fn flag_name(self) -> &'static str {
        match self {
            Role::Red => "red",
            Role::Green => "green",
            Role::Blue => "blue",
            Role::Alpha => "alpha",
        }
    }
}

/// A one-row-at-a-time supply of a single channel-source image's pixel
/// data, so combining up to four source images into one output never needs
/// to hold any of them fully in memory.
pub trait RowSource {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn channels(&self) -> usize;
    /// True for single-channel or single-channel-plus-alpha sources, where
    /// there's no distinct R/G/B to pick between.
    fn is_gray(&self) -> bool;
    /// True if this source's native layout has an alpha channel.
    fn has_alpha(&self) -> bool;
    fn next_row(&mut self) -> Result<Option<Vec<u8>>, ProcessError>;
}

/// Picks the byte(s) for `role` out of one already-decoded row.
///
/// A color source contributes its same-named channel (an image passed as
/// `--red` gives up its R channel, `--blue` its B channel, and so on). A
/// grayscale source has no R/G/B/A distinction, so its one channel is used
/// for whichever role it was assigned — *except* when it's grayscale+alpha
/// and assigned to the alpha role, where its real alpha channel is used
/// instead of its luma. A non-alpha color source (RGB) assigned to the
/// alpha role has no alpha data to give, which is an error rather than a
/// silent default (an unintentional 100% alpha or 0% alpha are both
/// surprising).
pub fn extract_role(
    source: &dyn RowSource,
    row: &[u8],
    role: Role,
) -> Result<Vec<u8>, ProcessError> {
    let channels = source.channels();
    let offset = match (source.is_gray(), role) {
        (true, Role::Alpha) if source.has_alpha() => 1,
        (true, _) => 0,
        (false, Role::Red) => 0,
        (false, Role::Green) => 1,
        (false, Role::Blue) => 2,
        (false, Role::Alpha) if source.has_alpha() => 3,
        (false, Role::Alpha) => {
            return Err(ProcessError::InvalidInput(
                "`--alpha` source has no alpha channel to extract (pass an RGBA/grayscale+alpha \
                 image, or omit `--alpha` for RGB output)"
                    .to_string(),
            ));
        }
    };
    Ok(row.iter().skip(offset).step_by(channels).copied().collect())
}

/// Opens `path` as a `RowSource` if it's a PNG or TIFF in the row/strip-
/// streamable shape `PngRowSource`/`TiffRowSource` support. Returns
/// `Ok(None)` (not an error) for anything else — a different format
/// entirely, or a PNG/TIFF outside that shape — so callers can fall back to
/// a fully buffered decode for the whole operation.
pub fn open_row_source(path: &Path) -> Result<Option<Box<dyn RowSource>>, ProcessError> {
    if crate::utils::is_png(path)? {
        return Ok(PngRowSource::open(path)?.map(|s| Box::new(s) as Box<dyn RowSource>));
    }
    if crate::utils::is_tiff(path)? {
        return Ok(TiffRowSource::open(path)?.map(|s| Box::new(s) as Box<dyn RowSource>));
    }
    Ok(None)
}

pub struct PngRowSource {
    reader: png::Reader<BufReader<File>>,
    width: u32,
    height: u32,
    channels: usize,
    is_gray: bool,
    has_alpha: bool,
}

impl PngRowSource {
    /// Returns `Ok(None)` (not an error) for PNGs that don't fit the simple
    /// row-streaming shape `pixel_stream::stream_png_pointwise` also relies
    /// on: interlaced, indexed color, or >8-bit depth.
    pub fn open(path: &Path) -> Result<Option<Self>, ProcessError> {
        let decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        let reader = decoder.read_info()?;
        let info = reader.info();
        if info.interlaced
            || info.bit_depth != png::BitDepth::Eight
            || info.color_type == png::ColorType::Indexed
        {
            return Ok(None);
        }

        let channels = info.color_type.samples();
        let is_gray = matches!(
            info.color_type,
            png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha
        );
        let has_alpha = matches!(
            info.color_type,
            png::ColorType::GrayscaleAlpha | png::ColorType::Rgba
        );
        let (width, height) = (info.width, info.height);

        Ok(Some(PngRowSource {
            reader,
            width,
            height,
            channels,
            is_gray,
            has_alpha,
        }))
    }
}

impl RowSource for PngRowSource {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn channels(&self) -> usize {
        self.channels
    }

    fn is_gray(&self) -> bool {
        self.is_gray
    }

    fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    fn next_row(&mut self) -> Result<Option<Vec<u8>>, ProcessError> {
        Ok(self.reader.next_row()?.map(|row| row.data().to_vec()))
    }
}

pub struct TiffRowSource {
    decoder: tiff::decoder::Decoder<BufReader<File>>,
    width: u32,
    height: u32,
    channels: usize,
    is_gray: bool,
    has_alpha: bool,
    rows_per_strip: u32,
    strip_count: u32,
    next_strip_index: u32,
    current_strip: Vec<u8>,
    row_in_strip: usize,
    rows_in_current_strip: usize,
}

impl TiffRowSource {
    /// Returns `Ok(None)` (not an error) for TIFFs that don't fit the
    /// simple strip-streaming shape `pixel_stream::stream_tiff_pointwise`
    /// also relies on: tiled (rather than stripped) layouts, planar
    /// (non-interleaved) channel storage, and anything other than 8-bit
    /// Grayscale/RGB/RGBA samples.
    pub fn open(path: &Path) -> Result<Option<Self>, ProcessError> {
        let mut decoder = tiff::decoder::Decoder::new(BufReader::new(File::open(path)?))?;

        let is_chunky = decoder
            .find_tag_unsigned::<u16>(tiff::tags::Tag::PlanarConfiguration)?
            .unwrap_or(1)
            == 1;
        if decoder.get_chunk_type() != tiff::decoder::ChunkType::Strip || !is_chunky {
            return Ok(None);
        }

        let (width, height) = decoder.dimensions()?;
        let color_type = decoder.colortype()?;
        let channels = match color_type {
            tiff::ColorType::Gray(8) => 1,
            tiff::ColorType::RGB(8) => 3,
            tiff::ColorType::RGBA(8) => 4,
            _ => return Ok(None),
        };
        let is_gray = matches!(color_type, tiff::ColorType::Gray(8));
        let has_alpha = matches!(color_type, tiff::ColorType::RGBA(8));

        let strip_count = decoder.strip_count()?;
        let rows_per_strip = decoder
            .find_tag_unsigned::<u32>(tiff::tags::Tag::RowsPerStrip)?
            .unwrap_or(height);

        Ok(Some(TiffRowSource {
            decoder,
            width,
            height,
            channels,
            is_gray,
            has_alpha,
            rows_per_strip,
            strip_count,
            next_strip_index: 0,
            current_strip: Vec::new(),
            row_in_strip: 0,
            rows_in_current_strip: 0,
        }))
    }

    /// Decodes the next strip, if any, into `current_strip` and resets the
    /// row cursor. This is what lets `next_row` hand back rows one at a
    /// time regardless of how many rows each strip happens to hold — the
    /// caller never needs to know or match this source's strip boundaries.
    fn load_next_strip(&mut self) -> Result<bool, ProcessError> {
        if self.next_strip_index >= self.strip_count {
            return Ok(false);
        }

        let tiff::decoder::DecodingResult::U8(strip) =
            self.decoder.read_chunk(self.next_strip_index)?
        else {
            unreachable!("colortype() reported an 8-bit sample type")
        };
        let rows_in_strip = min(
            self.rows_per_strip,
            self.height - self.next_strip_index * self.rows_per_strip,
        ) as usize;

        self.current_strip = strip;
        self.rows_in_current_strip = rows_in_strip;
        self.row_in_strip = 0;
        self.next_strip_index += 1;
        Ok(true)
    }
}

impl RowSource for TiffRowSource {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn channels(&self) -> usize {
        self.channels
    }

    fn is_gray(&self) -> bool {
        self.is_gray
    }

    fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    fn next_row(&mut self) -> Result<Option<Vec<u8>>, ProcessError> {
        if self.row_in_strip >= self.rows_in_current_strip && !self.load_next_strip()? {
            return Ok(None);
        }

        let row_len = self.width as usize * self.channels;
        let start = self.row_in_strip * row_len;
        let row = self.current_strip[start..start + row_len].to_vec();
        self.row_in_strip += 1;
        Ok(Some(row))
    }
}
