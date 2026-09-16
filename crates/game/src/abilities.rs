use bevy::prelude::*;

use crate::{
    blasters::{advance_shots, shoot_authoritative_players, shoot_predicted_players},
    movement::{move_authoritative_players, move_predicted_players},
};

/*
Shared ability entry points. Future phase-beam and boost systems belong here;
weapon-specific projectile behavior stays in its own module.
*/
pub(super) struct ClientAbilitiesPlugin;
pub(super) struct ServerAbilitiesPlugin;

impl Plugin for ClientAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            shoot_predicted_players.after(move_predicted_players),
        );
    }
}

impl Plugin for ServerAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (advance_shots, shoot_authoritative_players)
                .chain()
                .after(move_authoritative_players),
        );
    }
}
