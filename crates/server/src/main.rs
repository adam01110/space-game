mod network;
mod player;

use crate::{
    network::{prepare_client_link, spawn_server, start_server},
    player::spawn_player_for_client,
};
use bevy::{prelude::*, state::app::StatesPlugin};
use lightyear::prelude::{ReplicationMetadata, server::*};
use project_game::{GamePlugin, ServerSimulationPlugin};
use project_protocol::ProtocolPlugin;
use std::time::Duration;

const SERVER_UPS: f64 = 60.0;

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / SERVER_UPS),
            )),
            StatesPlugin,
            ServerPlugins::default(),
            ProtocolPlugin,
            GamePlugin,
            ServerSimulationPlugin,
        ))
        .insert_resource(ReplicationMetadata::new(Duration::from_millis(50)))
        .add_systems(Startup, (spawn_server, start_server).chain())
        .add_observer(prepare_client_link)
        .add_observer(spawn_player_for_client)
        .run();
}
