use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::descriptor::CountryId;

use super::Industries;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CityDescriptor {
    pub id: u32,
    pub lat: f64,
    pub lon: f64,
    pub name: SmolStr,
    pub country: CountryId,
    pub scale_rank: u8,
    pub label_rank: u8,
    pub population: u32,
    pub industries: Industries,
}
