use bevy::prelude::*;
use metrics::CumulativeMetric;
use scenario::Scenario;

use crate::GameScenario;

pub fn spawn_combatants(
    mut commands: Commands,
    scenario: Res<GameScenario>,
    scenarios: Res<Assets<Scenario>>,
) {
    let scenario = scenarios.get(&scenario.0).unwrap();

    for combatant in scenario.combatants.iter() {
        commands.spawn((
            *combatant,
            CumulativeMetric::<"damage">::default(),
            CumulativeMetric::<"unrest">::default(),
            CumulativeMetric::<"activity">::default(),
        ));
    }
}
