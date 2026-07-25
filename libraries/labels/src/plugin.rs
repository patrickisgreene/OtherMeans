use bevy::prelude::*;

use super::{LabelsRoot, systems};

pub struct LabelsPlugin<S: States + Copy> {
    state: S,
}

impl<S: States + Copy> LabelsPlugin<S> {
    pub fn new(state: S) -> Self {
        LabelsPlugin { state }
    }
}

impl<S: States + Copy> Plugin for LabelsPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(self.state), spawn_labels_root);
        app.add_systems(
            Update,
            (systems::update_labels, systems::update_positions)
                .chain()
                .run_if(in_state(self.state)),
        );
    }
}

fn spawn_labels_root(mut commands: Commands) {
    commands.spawn((Name::new("Labels"), LabelsRoot));
}
