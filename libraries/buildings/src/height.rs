use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext},
    prelude::*,
};
use thiserror::Error;

/// Geotransform of `assets/earth/height.tif`, produced by
/// `resources/earth/scripts/heightmap.sh` via `gdal_translate -outsize 5400 2700 ...` on a
/// full-globe (-180..180, -90..90) source. Must stay in sync with that script's
/// `-outsize`/extent if it ever changes.
const ORIGIN_LON: f64 = -180.0;
const ORIGIN_LAT: f64 = 90.0;
const PIXEL_SIZE_LON: f64 = 360.0 / 5400.0;
const PIXEL_SIZE_LAT: f64 = 180.0 / 2700.0;

/// A coarse, whole-globe elevation raster (grayscale, row-major), sampled on the CPU to place
/// buildings on the actual terrain surface instead of the base ellipsoid.
///
/// Values are normalized (byte/255, i.e. `[0, 1]`), matching the same `(EPSILON, 1.0]`-for-land
/// / `0.0`-for-ocean encoding as `resources/earth/height.tif` (see `heightmap.sh`) - this is
/// the exact same source data the terrain mesh itself is generated from, so sampling it here
/// keeps buildings visually consistent with the actual rendered ground rather than some
/// independently "corrected" elevation. Callers must multiply by the terrain's runtime
/// `height_scale` (`TileAtlas::height_scale`) to get the same real-world metres the terrain
/// mesh displaces by.
#[derive(Asset, TypePath)]
pub struct HeightMap {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl HeightMap {
    /// Returns the normalized elevation (`[0, 1]`, 0 = ocean/sea level) at the given
    /// latitude/longitude, or 0.0 if outside the raster's coverage.
    pub fn sample(&self, lat_deg: f64, lon_deg: f64) -> f32 {
        let col = (lon_deg - ORIGIN_LON) / PIXEL_SIZE_LON;
        let row = (ORIGIN_LAT - lat_deg) / PIXEL_SIZE_LAT;

        if col < 0.0 || row < 0.0 {
            return 0.0;
        }

        let (col, row) = (col as u32, row as u32);
        if col >= self.width || row >= self.height {
            return 0.0;
        }

        self.pixels[(row * self.width + col) as usize] as f32 / 255.0
    }
}

#[derive(Default, TypePath)]
pub struct HeightMapLoader;

#[derive(Debug, Error)]
pub enum HeightMapLoaderError {
    #[error("Could not read height map texture: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode height map texture: {0}")]
    Image(#[from] image::ImageError),
}

impl AssetLoader for HeightMapLoader {
    type Asset = HeightMap;
    type Settings = ();
    type Error = HeightMapLoaderError;

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

        Ok(HeightMap {
            width,
            height,
            pixels: image.into_raw(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tif", "tiff"]
    }
}
