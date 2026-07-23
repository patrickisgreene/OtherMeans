use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::HashMap;

use crate::{CombatantId, ControlledTerritory, StatisticModifier, WeaponsPlacementStrategy};
use cities::descriptor::CountryId;

#[derive(Resource, Default, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub combatants: HashSet<CombatantId>,
    pub territory: HashMap<CombatantId, Vec<ControlledTerritory>>,
    pub effect_modifiers: HashMap<CombatantId, HashMap<SmolStr, StatisticModifier>>,
    pub weapons_placement: WeaponsPlacementStrategy,
}

impl Scenario {
    pub fn new_with_player() -> Scenario {
        let mut combatants = HashSet::new();
        combatants.insert(CombatantId::PLAYER);
        Scenario {
            combatants,
            ..Default::default()
        }
    }

    pub fn players_america() -> Scenario {
        let mut scene = Self::new_with_player();
        scene.territory.insert(
            CombatantId::PLAYER,
            vec![ControlledTerritory::Country(['U', 'S', 'A'])],
        );
        scene
    }

    pub fn combatant_for_place_or_country(
        &self,
        index: usize,
        country: &CountryId,
    ) -> Option<CombatantId> {
        self.territory
            .iter()
            .find(|(_, territories)| {
                territories.iter().any(|t| {
                    *t == ControlledTerritory::Place(index)
                        || *t == ControlledTerritory::Country(country.0)
                })
            })
            .map(|(id, _)| *id)
    }
}
