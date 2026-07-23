use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use derive_more::From;

use crate::data::Weapon;

#[derive(Component, Default, From, DerefMut, Deref)]
pub struct Aresenal {
    inner: HashMap<Handle<Weapon>, usize>,
}
