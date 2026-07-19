use serde::{Deserialize, Serialize};
use std::{fmt::Error, str::FromStr};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub enum AttachmentLabel {
    #[default]
    Height,
    Custom(smol_str::SmolStr),
    Empty(usize),
}

impl From<&AttachmentLabel> for String {
    fn from(value: &AttachmentLabel) -> Self {
        match value {
            AttachmentLabel::Height => "height".into(),
            AttachmentLabel::Custom(name) => name.to_string(),
            AttachmentLabel::Empty(i) => format!("empty_{}", (b'a' + *i as u8) as char),
        }
    }
}

impl FromStr for AttachmentLabel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "height" => Ok(Self::Height),
            name => Ok(Self::Custom(name.into())),
        }
    }
}
