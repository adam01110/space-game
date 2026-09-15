mod abilities;
mod auth;
mod network;
mod player;
mod plugins;
mod security;

use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin};
use lightyear::prelude::{ReplicationMetadata, server::*};

use project_game::{GamePlugin, SERVER_UPS, ServerSimulationPlugin};
use project_protocol::ProtocolPlugin;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(server) = plugins::configure()? else {
        return Ok(());
    };

    App::new()
        .add_plugins((
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / SERVER_UPS),
            )),
            bevy::log::LogPlugin {
                // RUST_LOG can override the default level/filter.
                filter: "info,wgpu=error,naga=warn".into(),
                ..default()
            },
            StatesPlugin,
            bevy::transform::TransformPlugin,
            ServerPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
            },
            ProtocolPlugin,
            GamePlugin,
            ServerSimulationPlugin,
            server,
        ))
        .insert_resource(ReplicationMetadata::new(Duration::from_secs_f64(
            1.0 / SERVER_UPS,
        )))
        .run();
    Ok(())
}
