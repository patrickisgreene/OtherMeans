use bevy::prelude::*;

use crate::*;

pub fn init_cumulative_components_simple<const S: &'static str>(
    statistics: Query<&Metric<S>>,
    mut cumulative: Query<&mut CumulativeMetric<S>, Added<CumulativeMetric<S>>>,
) {
    for mut cumu in cumulative.iter_mut() {
        for stat in statistics.iter() {
            cumu.value += stat.value as u32;
            cumu.max += Metric::<S>::MAX as u32;
        }
    }
}

pub fn init_cumulative_components_tagged<const S: &'static str, T: Component + PartialEq>(
    statistics: Query<(&Metric<S>, &T)>,
    mut cumulative: Query<(&mut CumulativeMetric<S>, &T), Added<CumulativeMetric<S>>>,
) {
    for (mut cumu, tag) in cumulative.iter_mut() {
        for (stat, id) in statistics {
            if tag == id {
                cumu.value += stat.value as u32;
                cumu.max = Metric::<S>::MAX as u32;
            }
        }
    }
}

pub fn init_components_simple<const S: &'static str>(
    statistics: Query<&Metric<S>, Added<Metric<S>>>,
    mut cumulative: Query<&mut CumulativeMetric<S>>,
) {
    if statistics.is_empty() {
        return;
    }
    for mut cumu in cumulative.iter_mut() {
        for stat in statistics.iter() {
            cumu.value += stat.value as u32;
            cumu.max += Metric::<S>::MAX as u32;
        }
    }
}

pub fn init_components_tagged<const S: &'static str, T: Component + PartialEq>(
    statistics: Query<(&Metric<S>, &T), Added<Metric<S>>>,
    mut cumulative: Query<(&mut CumulativeMetric<S>, &T)>,
) {
    for (stat, tag) in statistics.iter() {
        for (mut cumu, tag2) in cumulative.iter_mut() {
            if tag == tag2 {
                cumu.value += stat.value as u32;
                cumu.max += Metric::<S>::MAX as u32;
            }
        }
    }
}

pub fn update_components_simple<const S: &'static str>(
    statistics: Query<(Entity, &Metric<S>), Changed<Metric<S>>>,
    added: Query<Entity, Added<Metric<S>>>,
    mut cumulative: Query<&mut CumulativeMetric<S>>,
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
    statistics: Query<(Entity, &Metric<S>, &T), Changed<Metric<S>>>,
    added: Query<Entity, Added<Metric<S>>>,
    mut cumulative: Query<(&mut CumulativeMetric<S>, &T)>,
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
    mut removed: RemovedComponents<Metric<S>>,
    statistics: Query<&Metric<S>>,
    mut cumulative: Query<&mut CumulativeMetric<S>>,
) {
    if removed.read().next().is_none() {
        return;
    }
    for mut cumu in cumulative.iter_mut() {
        cumu.value = statistics.iter().map(|s| s.value as u32).sum();
        cumu.max = statistics.iter().count() as u32 * Metric::<S>::MAX as u32;
    }
}

pub fn remove_component_tagged<const S: &'static str, T: Component + PartialEq>(
    mut removed: RemovedComponents<Metric<S>>,
    statistics: Query<(&Metric<S>, &T)>,
    mut cumulative: Query<(&mut CumulativeMetric<S>, &T)>,
) {
    if removed.read().next().is_none() {
        return;
    }
    for (mut cumu, tag) in cumulative.iter_mut() {
        let matching: Vec<_> = statistics.iter().filter(|(_, t)| *t == tag).collect();
        cumu.value = matching.iter().map(|(s, _)| s.value as u32).sum();
        cumu.max = matching.len() as u32 * Metric::<S>::MAX as u32;
    }
}
