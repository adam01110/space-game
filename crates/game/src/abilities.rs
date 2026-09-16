use bevy::prelude::*;

use crate::{
    blasters::{advance_shots, shoot_authoritative_players, shoot_predicted_players},
    movement::{move_authoritative_players, move_predicted_players},
    phase_beam::{beam_authoritative_players, beam_predicted_players},
};

// Shared ability entry points. Weapon-specific behavior stays in its own module.
pub(super) struct ClientAbilitiesPlugin;
pub(super) struct ServerAbilitiesPlugin;

impl Plugin for ClientAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (shoot_predicted_players, beam_predicted_players).after(move_predicted_players),
        );
    }
}

impl Plugin for ServerAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                (advance_shots, shoot_authoritative_players).chain(),
                beam_authoritative_players,
            )
                .after(move_authoritative_players),
        );
    }
}
