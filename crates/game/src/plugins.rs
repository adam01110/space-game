use bevy::prelude::*;

use crate::{
    abilities::{ClientAbilitiesPlugin, ServerAbilitiesPlugin},
    movement::{move_authoritative_players, move_predicted_players},
    phase_beam::{beam_authoritative_players, beam_predicted_players},
    physics::{install_physics, prepare_authoritative_body, prepare_predicted_body},
};

pub const SERVER_UPS: f64 = 60.0;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(SERVER_UPS));
        install_physics(app);
    }
}

pub struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_predicted_body);
        // Input is buffered in FixedPreUpdate. Apply desired velocity here, then Avian
        // resolves contacts in FixedPostUpdate before Lightyear records prediction history.
        app.add_systems(FixedUpdate, move_predicted_players)
            .add_systems(
                FixedUpdate,
                beam_predicted_players.after(move_predicted_players),
            )
            .add_plugins(ClientAbilitiesPlugin);
    }
}

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_authoritative_body);
        app.add_systems(FixedUpdate, move_authoritative_players)
            .add_systems(
                FixedUpdate,
                beam_authoritative_players.after(move_authoritative_players),
            )
            .add_plugins(ServerAbilitiesPlugin);
    }
}
