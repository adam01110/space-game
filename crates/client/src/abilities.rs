use bevy::prelude::*;

use crate::{
    blasters::{add_shot_visuals, update_shot_visuals},
    phase_beam::{add_phase_beam_visuals, update_phase_beam_visuals},
};

// Central registration for ability presentation, including future boost effects.
pub(super) struct ClientAbilitiesPlugin;

impl Plugin for ClientAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (add_shot_visuals, update_shot_visuals).chain());
        app.add_systems(
            Update,
            (add_phase_beam_visuals, update_phase_beam_visuals).chain(),
        );
    }
}
