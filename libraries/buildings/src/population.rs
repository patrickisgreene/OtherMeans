use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext},
    prelude::*,
};
use thiserror::Error;

/// Geotransform of `assets/earth/population.tif`, produced by
/// `resources/earth/scripts/population.sh` via
/// `gdal_translate -outsize 5400 2700 ...` on a WGS84-equirectangular source.
/// Must stay in sync with that script's `-outsize`/extent if it ever changes.
const ORIGIN_LON: f64 = -180.0;
const ORIGIN_LAT: f64 = 84.0;
const PIXEL_SIZE_LON: f64 = 360.0 / 5400.0;
const PIXEL_SIZE_LAT: f64 = 144.0 / 2700.0;

/// A coarse, whole-globe population-density raster (grayscale, row-major),
/// sampled on the CPU to decide where to place buildings and how tall to make them.
#[derive(Asset, TypePath)]
pub struct PopulationDensity {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl PopulationDensity {
    /// Returns the raw density byte (0-255, gamma-compressed) at the given latitude/longitude,
    /// or 0 if outside the raster's coverage (correctly treats the clipped polar regions, and
    /// any out-of-range input, as unpopulated).
    pub fn sample(&self, lat_deg: f64, lon_deg: f64) -> u8 {
        let col = (lon_deg - ORIGIN_LON) / PIXEL_SIZE_LON;
        let row = (ORIGIN_LAT - lat_deg) / PIXEL_SIZE_LAT;

        if col < 0.0 || row < 0.0 {
            return 0;
        }

        let (col, row) = (col as u32, row as u32);
        if col >= self.width || row >= self.height {
            return 0;
        }

        self.pixels[(row * self.width + col) as usize]
    }
}

#[derive(Default, TypePath)]
pub struct PopulationDensityLoader;

#[derive(Debug, Error)]
pub enum PopulationDensityLoaderError {
    #[error("Could not read population density texture: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode population density texture: {0}")]
    Image(#[from] image::ImageError),
}

impl AssetLoader for PopulationDensityLoader {
    type Asset = PopulationDensity;
    type Settings = ();
    type Error = PopulationDensityLoaderError;

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

        Ok(PopulationDensity {
            width,
            height,
            pixels: image.into_raw(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tif", "tiff"]
    }
}
