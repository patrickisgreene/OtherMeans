use clap::ValueEnum;
use derive_more::Display;
use image::ImageFormat;

#[derive(Display, Debug, PartialEq, Default, Clone, Copy, Hash, Eq, ValueEnum)]
pub enum TextureFormat {
    #[default]
    #[display("png")]
    Png,
    #[display("tiff")]
    Tiff,
}

impl From<TextureFormat> for ImageFormat {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::Png => ImageFormat::Png,
            TextureFormat::Tiff => ImageFormat::Tiff,
        }
    }
}

#[derive(Display, Debug, PartialEq, Default, Clone, Copy, Hash, Eq, ValueEnum)]
pub enum DisplayFormat {
    #[default]
    #[display("console")]
    Console,
    #[display("json")]
    Json,
}
