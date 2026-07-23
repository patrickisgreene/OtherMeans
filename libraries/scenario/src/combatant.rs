use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Component, PartialEq, Clone, Copy, Hash, Eq, Serialize, Deserialize)]
pub struct CombatantId(usize);

impl CombatantId {
    pub const PLAYER: CombatantId = CombatantId(0);
}

pub struct CombatantGenerator {
    inner: usize,
}

impl CombatantGenerator {
    pub fn new() -> CombatantGenerator {
        CombatantGenerator { inner: 1 }
    }

    pub fn get(&mut self) -> CombatantId {
        self.inner += 1;
        CombatantId(self.inner - 1)
    }
}
