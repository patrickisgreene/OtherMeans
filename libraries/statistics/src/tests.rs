use super::*;
use bevy::prelude::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(StatisticsPlugin::<"testing">::default());
    app
}

fn spawn(app: &mut App, value: u8) -> Entity {
    app.world_mut()
        .spawn(Statistic::<"testing"> {
            last: 0,
            value: value,
        })
        .id()
}

fn spawn_cumu(app: &mut App) {
    app.world_mut()
        .spawn(CumulativeStatistic::<"testing">::default());
}

fn cumu_value(app: &mut App) -> u32 {
    app.world_mut()
        .query::<&CumulativeStatistic<"testing">>()
        .single(app.world())
        .map(|x| x.get())
        .unwrap()
}

fn cumu_max(app: &mut App) -> u32 {
    app.world_mut()
        .query::<&CumulativeStatistic<"testing">>()
        .single(app.world())
        .map(|x| x.max)
        .unwrap()
}

#[test]
fn on_cumulative_added() {
    let mut app = app();

    spawn(&mut app, 100);
    spawn(&mut app, 200);
    // Since the components are expected to be inserted through out the apps life cycle.
    // We must update here after inserting to prevent getting duplicate entries when multiple components
    // are inserted.
    app.update();

    spawn_cumu(&mut app);

    app.update();

    let cumu = cumu_value(&mut app);

    assert_eq!(cumu, 300);
}

#[test]
fn on_statistic_added() {
    let mut app = app();

    spawn_cumu(&mut app);

    app.update();

    spawn(&mut app, 150);
    spawn(&mut app, 5);
    spawn(&mut app, 15);

    app.update();

    let cumu = cumu_value(&mut app);

    assert_eq!(cumu, 170);
}

#[test]
fn statistic_value_updated() {
    let mut app = app();

    let a = spawn(&mut app, 100);
    spawn(&mut app, 200);
    // Since the components are expected to be inserted through out the apps life cycle.
    // We must update here after inserting to prevent getting duplicate entries when multiple components
    // are inserted.
    app.update();

    spawn_cumu(&mut app);

    app.update();

    assert_eq!(cumu_value(&mut app), 300);

    app.world_mut()
        .get_mut::<Statistic<"testing">>(a)
        .unwrap()
        .set(150);

    app.update();

    assert_eq!(cumu_value(&mut app), 350);
}

#[test]
fn on_statistic_removed() {
    let mut app = app();

    let a = spawn(&mut app, 100);
    spawn(&mut app, 200);

    app.update();

    spawn_cumu(&mut app);

    app.update();

    assert_eq!(cumu_value(&mut app), 300);
    assert_eq!(cumu_max(&mut app), 2 * Statistic::<"testing">::MAX as u32);

    app.world_mut()
        .entity_mut(a)
        .remove::<Statistic<"testing">>();

    app.update();

    assert_eq!(cumu_value(&mut app), 200);
    assert_eq!(cumu_max(&mut app), Statistic::<"testing">::MAX as u32);
}
