use bevy::prelude::*;

use crate::{
    movement::{move_authoritative_players, move_predicted_players},
    transform::sync_player_transforms,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_player_transforms);
    }
}

pub struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_predicted_players);
    }
}

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_authoritative_players);
    }
}
