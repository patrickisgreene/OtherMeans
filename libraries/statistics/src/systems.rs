use bevy::prelude::*;

use crate::*;

pub fn init_cumulative_components_simple<const S: &'static str>(
    statistics: Query<&Statistic<S>>,
    mut cumulative: Query<&mut CumulativeStatistic<S>, Added<CumulativeStatistic<S>>>,
) {
    for mut cumu in cumulative.iter_mut() {
        for stat in statistics.iter() {
            cumu.value += stat.value as u32;
            cumu.max += Statistic::<S>::MAX as u32;
        }
    }
}

pub fn init_cumulative_components_tagged<const S: &'static str, T: Component + PartialEq>(
    statistics: Query<(&Statistic<S>, &T)>,
    mut cumulative: Query<(&mut CumulativeStatistic<S>, &T), Added<CumulativeStatistic<S>>>,
) {
    for (mut cumu, tag) in cumulative.iter_mut() {
        for (stat, id) in statistics {
            if tag == id {
                cumu.value += stat.value as u32;
                cumu.max = Statistic::<S>::MAX as u32;
            }
        }
    }
}

pub fn init_components_simple<const S: &'static str>(
    statistics: Query<&Statistic<S>, Added<Statistic<S>>>,
    mut cumulative: Query<&mut CumulativeStatistic<S>>,
) {
    if statistics.is_empty() {
        return;
    }
    for mut cumu in cumulative.iter_mut() {
        for stat in statistics.iter() {
            cumu.value += stat.value as u32;
            cumu.max += Statistic::<S>::MAX as u32;
        }
    }
}

pub fn init_components_tagged<const S: &'static str, T: Component + PartialEq>(
    statistics: Query<(&Statistic<S>, &T), Added<Statistic<S>>>,
    mut cumulative: Query<(&mut CumulativeStatistic<S>, &T)>,
) {
    for (stat, tag) in statistics.iter() {
        for (mut cumu, tag2) in cumulative.iter_mut() {
            if tag == tag2 {
                cumu.value += stat.value as u32;
                cumu.max += Statistic::<S>::MAX as u32;
            }
        }
    }
}

pub fn update_components_simple<const S: &'static str>(
    statistics: Query<(Entity, &Statistic<S>), Changed<Statistic<S>>>,
    added: Query<Entity, Added<Statistic<S>>>,
    mut cumulative: Query<&mut CumulativeStatistic<S>>,
) {
    for (entity, stat) in statistics.iter() {
        if added.contains(entity) {
            continue;
        }
        for mut cumu in cumulative.iter_mut() {
            cumu.value -= stat.last as u32;
            cumu.value += stat.value as u32;
        }
    }
}

pub fn update_components_tagged<const S: &'static str, T: Component + PartialEq>(
    statistics: Query<(Entity, &Statistic<S>, &T), Changed<Statistic<S>>>,
    added: Query<Entity, Added<Statistic<S>>>,
    mut cumulative: Query<(&mut CumulativeStatistic<S>, &T)>,
) {
    for (entity, stat, tag) in statistics.iter() {
        if added.contains(entity) {
            continue;
        }
        for (mut cumu, tag2) in cumulative.iter_mut() {
            if tag == tag2 {
                cumu.value -= stat.last as u32;
                cumu.value += stat.value as u32;
            }
        }
    }
}

pub fn remove_component_simple<const S: &'static str>(
    mut removed: RemovedComponents<Statistic<S>>,
    statistics: Query<&Statistic<S>>,
    mut cumulative: Query<&mut CumulativeStatistic<S>>,
) {
    if removed.read().next().is_none() {
        return;
    }
    for mut cumu in cumulative.iter_mut() {
        cumu.value = statistics.iter().map(|s| s.value as u32).sum();
        cumu.max = statistics.iter().count() as u32 * Statistic::<S>::MAX as u32;
    }
}

pub fn remove_component_tagged<const S: &'static str, T: Component + PartialEq>(
    mut removed: RemovedComponents<Statistic<S>>,
    statistics: Query<(&Statistic<S>, &T)>,
    mut cumulative: Query<(&mut CumulativeStatistic<S>, &T)>,
) {
    if removed.read().next().is_none() {
        return;
    }
    for (mut cumu, tag) in cumulative.iter_mut() {
        let matching: Vec<_> = statistics.iter().filter(|(_, t)| *t == tag).collect();
        cumu.value = matching.iter().map(|(s, _)| s.value as u32).sum();
        cumu.max = matching.len() as u32 * Statistic::<S>::MAX as u32;
    }
}
