use bevy::prelude::*;

use crate::{
    movement::{move_authoritative_players, move_predicted_players},
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
        app.add_systems(FixedUpdate, move_predicted_players);
    }
}

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_authoritative_body);
        app.add_systems(FixedUpdate, move_authoritative_players);
    }
}
