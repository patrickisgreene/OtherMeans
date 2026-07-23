mod combatant;
mod country;
mod scenario;
mod territory;
mod time;
mod weapons;

pub use combatant::*;
pub use country::*;
pub use scenario::*;
pub use territory::*;
pub use time::*;
pub use weapons::*;

pub type WeaponCount = usize;
pub type StatisticModifier = u8;
pub type PlaceId = u32;
