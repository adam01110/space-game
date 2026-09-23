use bevy::prelude::*;

use crate::{
    blasters::{
        advance_authoritative_shots, advance_predicted_shots, shoot_authoritative_players,
        shoot_predicted_players,
    },
    damage::{DamageConfig, apply_bullet_damage, apply_phase_beam_damage},
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
            (
                (advance_predicted_shots, shoot_predicted_players).chain(),
                beam_predicted_players,
            )
                .after(move_predicted_players),
        );
    }
}

impl Plugin for ServerAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DamageConfig>()
            .add_systems(
                FixedUpdate,
                (
                    (advance_authoritative_shots, shoot_authoritative_players).chain(),
                    beam_authoritative_players,
                )
                    .after(move_authoritative_players),
            )
            // Damage reads the poses and beam segments this tick produced, so it runs
            // after both weapons moved instead of against stale positions.
            .add_systems(
                FixedUpdate,
                (apply_bullet_damage, apply_phase_beam_damage)
                    .after(advance_authoritative_shots)
                    .after(beam_authoritative_players),
            );
    }
}
