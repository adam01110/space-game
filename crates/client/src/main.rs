mod camera;
mod guest;
mod input;
mod network;
mod player;
mod plugins;

use std::time::Duration;

use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use lightyear::prelude::client::*;

use crate::plugins::ClientAppPlugin;
use project_game::{ClientRenderingPlugin, ClientSimulationPlugin, GamePlugin, SERVER_UPS};
use project_protocol::ProtocolPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // Rendering follows the client's capabilities instead of the 60 Hz simulation.
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                }),
            ClientPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
            },
            ProtocolPlugin,
            GamePlugin,
            ClientSimulationPlugin,
            ClientRenderingPlugin,
            ClientAppPlugin,
        ))
        .run();
}
