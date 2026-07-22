use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext},
    prelude::*,
};
use thiserror::Error;

/// Geotransform of `assets/earth/ocean-mask.tif`, produced by
/// `resources/earth/scripts/ocean-mask.sh` via `texture-processor scale-image` on a
/// full-globe (-180..180, -90..90) source. Must stay in sync with that script's
/// `--width`/`--height` if it ever changes.
const ORIGIN_LON: f64 = -180.0;
const ORIGIN_LAT: f64 = 90.0;
const PIXEL_SIZE_LON: f64 = 360.0 / 5400.0;
const PIXEL_SIZE_LAT: f64 = 180.0 / 2700.0;
/// Byte value (0-255) at/above which a pixel is considered water, matching the `> 127`
/// convention already used by `resources/earth/scripts/heightmap.sh`.
const WATER_THRESHOLD: u8 = 128;

/// A coarse, whole-globe land/water mask (grayscale, row-major; 255 = ocean or lake, 0 = land),
/// sampled on the CPU to keep buildings off the water.
#[derive(Asset, TypePath)]
pub struct OceanMask {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl OceanMask {
    /// Returns whether the given latitude/longitude is water. Out-of-bounds lookups return
    /// `false` (not water) — this mask covers the whole globe, so that should never trigger.
    pub fn is_water(&self, lat_deg: f64, lon_deg: f64) -> bool {
        let col = (lon_deg - ORIGIN_LON) / PIXEL_SIZE_LON;
        let row = (ORIGIN_LAT - lat_deg) / PIXEL_SIZE_LAT;

        if col < 0.0 || row < 0.0 {
            return false;
        }

        let (col, row) = (col as u32, row as u32);
        if col >= self.width || row >= self.height {
            return false;
        }

        self.pixels[(row * self.width + col) as usize] >= WATER_THRESHOLD
    }
}

#[derive(Default, TypePath)]
pub struct OceanMaskLoader;

#[derive(Debug, Error)]
pub enum OceanMaskLoaderError {
    #[error("Could not read ocean mask texture: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode ocean mask texture: {0}")]
    Image(#[from] image::ImageError),
}

impl AssetLoader for OceanMaskLoader {
    type Asset = OceanMask;
    type Settings = ();
    type Error = OceanMaskLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let image = image::load_from_memory(&bytes)?.into_luma8();
        let (width, height) = image.dimensions();

        Ok(OceanMask {
            width,
            height,
            pixels: image.into_raw(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tif", "tiff"]
    }
}
