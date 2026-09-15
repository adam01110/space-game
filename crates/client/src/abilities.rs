use bevy::prelude::*;

use crate::blasters::{add_shot_visuals, update_shot_visuals};

// Central registration for ability presentation, including future phase beam and boost effects.
pub(super) struct ClientAbilitiesPlugin;

impl Plugin for ClientAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (add_shot_visuals, update_shot_visuals).chain());
    }
}
