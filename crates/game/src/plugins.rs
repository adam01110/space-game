use bevy::prelude::*;

use crate::{
    movement::{move_authoritative_players, move_predicted_players},
    transform::sync_player_transforms,
};

pub const SERVER_UPS: f64 = 60.0;

// Sets the simulation timestep independently of the client's render schedule.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(SERVER_UPS));
    }
}

// Copies simulation and interpolated network state into render transforms each client frame.
pub struct ClientRenderingPlugin;

impl Plugin for ClientRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_player_transforms);
    }
}

// Runs movement for entities with both `Player` and `Predicted` each fixed tick.
pub struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_predicted_players);
    }
}

// Runs movement for every entity with `Player` each fixed tick.
pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_authoritative_players);
    }
}
