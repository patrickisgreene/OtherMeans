#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
#[repr(C)]
pub enum MaterialData {
    Height = 1,
    Ocean = 2,
    Color = 3,
    Distance = 4,
    Bathyometry = 5,
    Chlorophyll = 6,
}

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
#[repr(C)]
pub enum RenderMode {
    Normal,
    Texture(MaterialData),
}

impl From<u32> for RenderMode {
    fn from(value: u32) -> Self {
        match value {
            0 => RenderMode::Normal,
            1 => RenderMode::Texture(MaterialData::Height),
            2 => RenderMode::Texture(MaterialData::Ocean),
            3 => RenderMode::Texture(MaterialData::Color),
            4 => RenderMode::Texture(MaterialData::Distance),
            5 => RenderMode::Texture(MaterialData::Bathyometry),
            6 => RenderMode::Texture(MaterialData::Chlorophyll),
            _ => panic!("Invalid Render Mode"),
        }
    }
}
