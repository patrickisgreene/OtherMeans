use gdal::errors::GdalError;

#[derive(Debug, derive_more::From)]
pub enum ProcessError {
    Io(std::io::Error),
    Image(image::ImageError),
    PngDecoding(png::DecodingError),
    PngEncoding(png::EncodingError),
    Tiff(tiff::TiffError),
    Gdal(GdalError),
    Shapefile(shapefile::Error),
    /// A user-facing validation failure that isn't a wrapped decoder/encoder
    /// error — e.g. mismatched channel-source dimensions, or a channel
    /// source with no alpha data to give.
    InvalidInput(String),
}
