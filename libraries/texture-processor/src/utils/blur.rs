use std::collections::VecDeque;
use std::path::Path;

use crate::format::DisplayFormat;
use crate::utils::{self, MaskSpec, Progress, RowSource, mask_byte_valid, mask_row_to_valid};
use crate::{ProcessError, TextureFormat};

/// Convolves one row of interleaved samples along x with a symmetric,
/// already-normalized 1D kernel (`weights.len()` is always odd), replicating
/// the first/last pixel for out-of-bounds taps (clamp-to-edge). Pure and
/// I/O-free, so it's directly unit-testable and reused identically by both
/// the streaming and buffered paths.
pub fn blur_row_horizontal(row: &[u8], channels: usize, weights: &[f32]) -> Vec<u8> {
    let radius = (weights.len() / 2) as i64;
    let width = (row.len() / channels) as i64;
    let mut out = vec![0u8; row.len()];
    for x in 0..width {
        for c in 0..channels {
            let mut acc = 0f32;
            for (k, w) in weights.iter().enumerate() {
                let dx = k as i64 - radius;
                let sx = (x + dx).clamp(0, width - 1) as usize;
                acc += w * f32::from(row[sx * channels + c]);
            }
            out[x as usize * channels + c] = acc.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// Anything that can hand back an already horizontally-blurred row by
/// absolute row index. Lets `blur_row_vertical` serve both a bounded
/// sliding window (`RowWindow`, streaming) and a fully materialized
/// `Vec<Vec<u8>>` (buffered) through identical weighted-sum logic —
/// guaranteeing byte-identical output between the two paths for the same
/// input+params.
trait HBlurredRows {
    fn row(&self, index: u32) -> &[u8];
}

impl HBlurredRows for Vec<Vec<u8>> {
    fn row(&self, index: u32) -> &[u8] {
        &self[index as usize]
    }
}

/// Computes one vertically-blurred output row (absolute row `y` of a
/// `height`-tall image) from `rows`, replicating row 0 / row `height-1`
/// for taps that land outside `[0, height)` — clamp-to-edge, matching
/// `blur_row_horizontal`'s x-axis policy.
fn blur_row_vertical(y: u32, height: u32, weights: &[f32], rows: &impl HBlurredRows) -> Vec<u8> {
    let radius = (weights.len() / 2) as i64;
    let mut acc: Vec<f32> = Vec::new();
    for (k, w) in weights.iter().enumerate() {
        let dy = k as i64 - radius;
        let sy = (y as i64 + dy).clamp(0, height as i64 - 1) as u32;
        let src = rows.row(sy);
        if acc.is_empty() {
            acc = vec![0f32; src.len()];
        }
        for (a, &b) in acc.iter_mut().zip(src) {
            *a += w * f32::from(b);
        }
    }
    acc.into_iter()
        .map(|v| v.round().clamp(0.0, 255.0) as u8)
        .collect()
}

/// A bounded window of horizontally-blurred rows, holding only the rows
/// needed to compute the next not-yet-emitted output row. Steady-state size
/// is `2*radius+1` rows regardless of image height — this is what makes
/// `stream_blur` use O(radius) memory instead of O(height), extending the
/// "buffer a bounded chunk, serve rows on demand" idea `TiffRowSource`
/// already uses for a fixed-size strip.
struct RowWindow<'w> {
    source: Box<dyn RowSource>,
    channels: usize,
    weights: &'w [f32],
    height: u32,
    rows: VecDeque<Vec<u8>>,
    front_index: u32,
    next_read_index: u32,
}

impl<'w> RowWindow<'w> {
    fn new(source: Box<dyn RowSource>, channels: usize, weights: &'w [f32], height: u32) -> Self {
        RowWindow {
            source,
            channels,
            weights,
            height,
            rows: VecDeque::new(),
            front_index: 0,
            next_read_index: 0,
        }
    }

    /// Pulls and horizontally-blurs rows from the source until the window
    /// contains every row row `y`'s vertical pass could need, i.e. up
    /// through `min(height-1, y+radius)`.
    fn ensure_upto(&mut self, y: u32) -> Result<(), ProcessError> {
        let radius = (self.weights.len() / 2) as u32;
        let target = (y + radius).min(self.height - 1);
        while self.next_read_index <= target {
            let raw = self.source.next_row()?.ok_or_else(|| {
                ProcessError::InvalidInput(format!(
                    "image source ended after {} of {} rows while blurring",
                    self.next_read_index, self.height
                ))
            })?;
            self.rows
                .push_back(blur_row_horizontal(&raw, self.channels, self.weights));
            self.next_read_index += 1;
        }
        Ok(())
    }

    /// Drops window rows with absolute index `< keep_from` — called right
    /// after emitting row `y` with `keep_from = (y+1).saturating_sub(radius)`,
    /// the lowest index row `y+1`'s vertical pass could still need.
    fn evict_before(&mut self, keep_from: u32) {
        while self.front_index < keep_from && !self.rows.is_empty() {
            self.rows.pop_front();
            self.front_index += 1;
        }
    }
}

impl HBlurredRows for RowWindow<'_> {
    fn row(&self, index: u32) -> &[u8] {
        &self.rows[(index - self.front_index) as usize]
    }
}

/// Opens `input` as a `RowSource` for the streaming blur path. Returns
/// `Ok(None)` (not an error) if `input` isn't PNG/TIFF-streamable at all
/// (see `open_row_source`), or if it's a 2-channel (grayscale+alpha) source
/// and TIFF output was requested — the `tiff` crate's typed encoder has no
/// grayscale+alpha color type, mirroring `binary_op::open_streamable_pair`'s
/// identical check. Callers should fall back to `buffered_blur`.
pub fn open_streamable_blur_source(
    input: &Path,
    format: TextureFormat,
) -> Result<Option<Box<dyn RowSource>>, ProcessError> {
    let Some(source) = utils::open_row_source(input)? else {
        return Ok(None);
    };
    let tiff_representable = format != TextureFormat::Tiff || source.channels() != 2;
    Ok(tiff_representable.then_some(source))
}

/// Streams a separable box/Gaussian blur: one horizontal convolution pass
/// per row as it's read, one vertical convolution pass per row as it's
/// emitted, using a `2*radius+1`-row sliding window — peak memory is
/// O(radius), not O(height).
pub fn stream_blur(
    source: Box<dyn RowSource>,
    weights: &[f32],
    output: &Path,
    format: TextureFormat,
    display_format: DisplayFormat,
    op_name: &'static str,
) -> Result<(), ProcessError> {
    let (width, height) = (source.width(), source.height());
    let (channels, is_gray, has_alpha) = (source.channels(), source.is_gray(), source.has_alpha());
    let radius = (weights.len() / 2) as u32;

    let mut window = RowWindow::new(source, channels, weights, height);
    let mut y = 0u32;
    let mut next_row = move || -> Result<Vec<u8>, ProcessError> {
        window.ensure_upto(y)?;
        let out = blur_row_vertical(y, height, weights, &window);
        window.evict_before((y + 1).saturating_sub(radius));
        y += 1;
        Ok(out)
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

/// Falls back to decoding fully via `image`, normalizing to RGBA8 (a
/// deliberate deviation from `buffered_fallback`'s "preserve the original
/// `DynamicImage` variant" convention — this hand-written convolution, like
/// `buffered_binary_op`'s, needs one concrete pixel layout to write simple
/// loop code against), then running the exact same `blur_row_horizontal`/
/// `blur_row_vertical` weighted-sum logic against a fully materialized
/// `Vec<Vec<u8>>` in place of the sliding window — guaranteeing
/// byte-identical output to `stream_blur` for the same input+params.
#[allow(clippy::too_many_arguments)]
pub fn buffered_blur(
    input: &Path,
    output: &Path,
    format: TextureFormat,
    no_memory_limit: bool,
    weights: &[f32],
    display_format: DisplayFormat,
    op_name: &'static str,
) -> Result<(), ProcessError> {
    let mut reader = image::ImageReader::open(input)?.with_guessed_format()?;
    if no_memory_limit {
        reader.no_limits();
    }
    let img = reader.decode()?.to_rgba8();
    let (width, height) = (img.width(), img.height());

    let h_blurred: Vec<Vec<u8>> = (0..height)
        .map(|y| {
            let row: Vec<u8> = (0..width).flat_map(|x| img.get_pixel(x, y).0).collect();
            blur_row_horizontal(&row, 4, weights)
        })
        .collect();

    let mut progress = Progress::new(display_format, op_name, Some(u64::from(height)));
    let mut out = image::RgbaImage::new(width, height);
    for y in 0..height {
        let row = blur_row_vertical(y, height, weights, &h_blurred);
        for x in 0..width {
            let i = x as usize * 4;
            *out.get_pixel_mut(x, y) = image::Rgba([row[i], row[i + 1], row[i + 2], row[i + 3]]);
        }
        progress.inc(1);
    }
    out.save_with_format(output, format.into())?;
    Ok(())
}

/// Like `blur_row_horizontal`, but computes normalized convolution's two
/// separable components instead of a single blurred value: `numerator` is
/// the weighted sum over only the *valid* neighbors' pixel values, and
/// `denominator` is the weighted sum of just their validity (1 or 0) — the
/// total "coverage" that window had. Dividing the two (after the vertical
/// pass does the same in the other dimension) gives a blur unaffected by
/// masked-out neighbors, instead of silently treating them as black.
pub fn blur_row_horizontal_masked(
    row: &[u8],
    valid: &[bool],
    channels: usize,
    weights: &[f32],
) -> (Vec<f32>, Vec<f32>) {
    let radius = (weights.len() / 2) as i64;
    let width = valid.len() as i64;
    let mut numerator = vec![0f32; row.len()];
    let mut denominator = vec![0f32; valid.len()];
    for x in 0..width {
        for (k, w) in weights.iter().enumerate() {
            let dx = k as i64 - radius;
            let sx = (x + dx).clamp(0, width - 1) as usize;
            if !valid[sx] {
                continue;
            }
            denominator[x as usize] += w;
            for c in 0..channels {
                numerator[x as usize * channels + c] += w * f32::from(row[sx * channels + c]);
            }
        }
    }
    (numerator, denominator)
}

/// One row's worth of state for masked blur: the horizontally-convolved
/// numerator/denominator (see `blur_row_horizontal_masked`), plus the raw
/// (unblurred) pixel row and this row's own per-pixel validity — needed at
/// emit time to pass a masked pixel's original value straight through.
struct MaskedRowData {
    numerator: Vec<f32>,
    denominator: Vec<f32>,
    raw: Vec<u8>,
    valid: Vec<bool>,
}

/// Anything that can hand back a `MaskedRowData` by absolute row index —
/// the masked-blur analog of `HBlurredRows`, serving both the bounded
/// `MaskedRowWindow` (streaming) and a fully materialized
/// `Vec<MaskedRowData>` (buffered) through identical vertical-pass logic.
trait MaskedRows {
    fn get(&self, index: u32) -> &MaskedRowData;
}

impl MaskedRows for Vec<MaskedRowData> {
    fn get(&self, index: u32) -> &MaskedRowData {
        &self[index as usize]
    }
}

/// Computes one output row for masked blur: vertically combines the
/// window's per-row numerator/denominator (clamp-to-edge on y, same policy
/// as the unmasked path), then per pixel either divides or passes the
/// original value through unchanged if the pixel itself is masked.
///
/// The `v_den[x] <= EPSILON` check is a defensive fallback, not a case
/// this algorithm can reach in practice: whenever a pixel is valid, its own
/// center tap (`dx=0`/`dy=0`, never clamped away since `x`/`y` are always
/// in range) always contributes a strictly positive amount to its own
/// denominator, so a valid pixel's coverage can only be exactly zero via
/// floating-point underflow, not "every neighbor happened to be masked."
fn blur_row_vertical_masked(
    y: u32,
    height: u32,
    weights: &[f32],
    channels: usize,
    rows: &impl MaskedRows,
) -> Vec<u8> {
    let radius = (weights.len() / 2) as i64;
    let width = rows.get(y).valid.len();

    let mut v_num = vec![0f32; width * channels];
    let mut v_den = vec![0f32; width];
    for (k, w) in weights.iter().enumerate() {
        let dy = k as i64 - radius;
        let sy = (y as i64 + dy).clamp(0, height as i64 - 1) as u32;
        let data = rows.get(sy);
        for (acc, &n) in v_num.iter_mut().zip(&data.numerator) {
            *acc += w * n;
        }
        for (acc, &d) in v_den.iter_mut().zip(&data.denominator) {
            *acc += w * d;
        }
    }

    let center = rows.get(y);
    let mut out = vec![0u8; width * channels];
    for x in 0..width {
        let use_original = !center.valid[x] || v_den[x] <= f32::EPSILON;
        for c in 0..channels {
            out[x * channels + c] = if use_original {
                center.raw[x * channels + c]
            } else {
                (v_num[x * channels + c] / v_den[x])
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
        }
    }
    out
}

/// The masked-blur analog of `RowWindow`: a bounded window of
/// `MaskedRowData`, reading the image and mask sources in lockstep. Same
/// O(radius) memory guarantee, just with more state carried per row.
struct MaskedRowWindow<'w> {
    image_source: Box<dyn RowSource>,
    mask_source: Box<dyn RowSource>,
    mask_channels: usize,
    mask_excludes_white: bool,
    channels: usize,
    weights: &'w [f32],
    height: u32,
    rows: VecDeque<MaskedRowData>,
    front_index: u32,
    next_read_index: u32,
}

impl<'w> MaskedRowWindow<'w> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        image_source: Box<dyn RowSource>,
        mask_source: Box<dyn RowSource>,
        mask_channels: usize,
        mask_excludes_white: bool,
        channels: usize,
        weights: &'w [f32],
        height: u32,
    ) -> Self {
        MaskedRowWindow {
            image_source,
            mask_source,
            mask_channels,
            mask_excludes_white,
            channels,
            weights,
            height,
            rows: VecDeque::new(),
            front_index: 0,
            next_read_index: 0,
        }
    }

    fn ensure_upto(&mut self, y: u32) -> Result<(), ProcessError> {
        let radius = (self.weights.len() / 2) as u32;
        let target = (y + radius).min(self.height - 1);
        while self.next_read_index <= target {
            let raw = self.image_source.next_row()?.ok_or_else(|| {
                ProcessError::InvalidInput(format!(
                    "image source ended after {} of {} rows while blurring",
                    self.next_read_index, self.height
                ))
            })?;
            let mask_row = self.mask_source.next_row()?.ok_or_else(|| {
                ProcessError::InvalidInput(format!(
                    "mask source ended after {} of {} rows while blurring",
                    self.next_read_index, self.height
                ))
            })?;
            let valid = mask_row_to_valid(&mask_row, self.mask_channels, self.mask_excludes_white);
            let (numerator, denominator) =
                blur_row_horizontal_masked(&raw, &valid, self.channels, self.weights);
            self.rows.push_back(MaskedRowData {
                numerator,
                denominator,
                raw,
                valid,
            });
            self.next_read_index += 1;
        }
        Ok(())
    }

    /// Drops window rows with absolute index `< keep_from` — see
    /// `RowWindow::evict_before`, identical logic.
    fn evict_before(&mut self, keep_from: u32) {
        while self.front_index < keep_from && !self.rows.is_empty() {
            self.rows.pop_front();
            self.front_index += 1;
        }
    }
}

impl MaskedRows for MaskedRowWindow<'_> {
    fn get(&self, index: u32) -> &MaskedRowData {
        &self.rows[(index - self.front_index) as usize]
    }
}

type RowSourcePair = (Box<dyn RowSource>, Box<dyn RowSource>);

/// Opens both the image and mask as `RowSource`s for the streaming masked-
/// blur path. Returns `Ok(None)` (not an error) if either isn't PNG/TIFF-
/// streamable, or if the image is a 2-channel (grayscale+alpha) source and
/// TIFF output was requested (mirrors `open_streamable_blur_source`'s
/// identical check). A dimension mismatch between image and mask is a hard
/// error either way, since there's no sensible way to align them.
pub fn open_streamable_masked_blur_sources(
    input: &Path,
    mask: MaskSpec,
    format: TextureFormat,
) -> Result<Option<RowSourcePair>, ProcessError> {
    let (Some(image_source), Some(mask_source)) = (
        utils::open_row_source(input)?,
        utils::open_row_source(mask.path)?,
    ) else {
        return Ok(None);
    };

    if (image_source.width(), image_source.height()) != (mask_source.width(), mask_source.height())
    {
        return Err(ProcessError::InvalidInput(format!(
            "mask `{}` is {}x{}, but image `{}` is {}x{} — the mask must match the image's dimensions",
            mask.path.display(),
            mask_source.width(),
            mask_source.height(),
            input.display(),
            image_source.width(),
            image_source.height(),
        )));
    }

    let tiff_representable = format != TextureFormat::Tiff || image_source.channels() != 2;
    Ok(tiff_representable.then_some((image_source, mask_source)))
}

/// Streams a masked separable box/Gaussian blur — see `stream_blur` for the
/// unmasked case and the module-level docs on `blur_row_horizontal_masked`/
/// `blur_row_vertical_masked` for how masking stays separable. Same
/// O(radius) memory guarantee via `MaskedRowWindow`.
#[allow(clippy::too_many_arguments)]
pub fn stream_masked_blur(
    image_source: Box<dyn RowSource>,
    mask_source: Box<dyn RowSource>,
    mask_excludes_white: bool,
    weights: &[f32],
    output: &Path,
    format: TextureFormat,
    display_format: DisplayFormat,
    op_name: &'static str,
) -> Result<(), ProcessError> {
    let (width, height) = (image_source.width(), image_source.height());
    let (channels, is_gray, has_alpha) = (
        image_source.channels(),
        image_source.is_gray(),
        image_source.has_alpha(),
    );
    let mask_channels = mask_source.channels();
    let radius = (weights.len() / 2) as u32;

    let mut window = MaskedRowWindow::new(
        image_source,
        mask_source,
        mask_channels,
        mask_excludes_white,
        channels,
        weights,
        height,
    );
    let mut y = 0u32;
    let mut next_row = move || -> Result<Vec<u8>, ProcessError> {
        window.ensure_upto(y)?;
        let out = blur_row_vertical_masked(y, height, weights, channels, &window);
        window.evict_before((y + 1).saturating_sub(radius));
        y += 1;
        Ok(out)
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

/// Falls back to decoding both the image and mask fully via `image`,
/// normalizing the image to RGBA8 (same reasoning as `buffered_blur`) and
/// reading the mask's own channel 0 per pixel (matching the streaming
/// path's `mask_row_to_valid` exactly, so the two paths stay in parity for
/// the same input+mask+params).
#[allow(clippy::too_many_arguments)]
pub fn buffered_masked_blur(
    input: &Path,
    mask: MaskSpec,
    output: &Path,
    format: TextureFormat,
    no_memory_limit: bool,
    weights: &[f32],
    display_format: DisplayFormat,
    op_name: &'static str,
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

    let rows: Vec<MaskedRowData> = (0..height)
        .map(|y| {
            let raw: Vec<u8> = (0..width).flat_map(|x| img.get_pixel(x, y).0).collect();
            let valid: Vec<bool> = (0..width)
                .map(|x| mask_byte_valid(mask_img.get_pixel(x, y).0[0], mask.excludes_white))
                .collect();
            let (numerator, denominator) = blur_row_horizontal_masked(&raw, &valid, 4, weights);
            MaskedRowData {
                numerator,
                denominator,
                raw,
                valid,
            }
        })
        .collect();

    let mut progress = Progress::new(display_format, op_name, Some(u64::from(height)));
    let mut out = image::RgbaImage::new(width, height);
    for y in 0..height {
        let row = blur_row_vertical_masked(y, height, weights, 4, &rows);
        for x in 0..width {
            let i = x as usize * 4;
            *out.get_pixel_mut(x, y) = image::Rgba([row[i], row[i + 1], row[i + 2], row[i + 3]]);
        }
        progress.inc(1);
    }
    out.save_with_format(output, format.into())?;
    Ok(())
}
