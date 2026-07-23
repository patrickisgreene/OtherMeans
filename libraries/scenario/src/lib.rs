mod combatant;
mod scenario;
mod territory;
mod weapons;

pub use combatant::*;
pub use scenario::*;
pub use territory::*;
pub use weapons::*;

pub type WeaponCount = usize;
pub type StatisticModifier = u8;
pub type PlaceId = u32;
