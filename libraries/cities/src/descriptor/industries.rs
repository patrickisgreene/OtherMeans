use bevy::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
#[repr(u32)]
pub enum Industry {
    Fishing = 0b00000001,
    Farming = 0b00000010,
    Mining = 0b00000100,
    Medical = 0b00001000,
    Tourism = 0b00010000,
    Shipping = 0b00100000,
    Technology = 0b01000000,
    Industrial = 0b10000000,
}

bitflags! {
    #[derive(Component, Debug, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
    pub struct Industries: u32 {
        const FISHING    = 0b00000001;
        const FARMING    = 0b00000010;
        const MINING     = 0b00000100;
        const MEDICAL    = 0b00001000;
        const TOURISM    = 0b00010000;
        const SHIPPING   = 0b00100000;
        const TECHNOLOGY = 0b01000000;
        const INDUSTRIAL = 0b10000000;
    }
}
