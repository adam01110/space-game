mod camera;
mod guest;
mod input;
mod network;
mod player;
mod plugins;

use bevy::prelude::*;
use lightyear::prelude::client::*;

use crate::plugins::ClientAppPlugin;
use project_game::{ClientSimulationPlugin, GamePlugin};
use project_protocol::ProtocolPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            ClientPlugins::default(),
            ProtocolPlugin,
            GamePlugin,
            ClientSimulationPlugin,
            ClientAppPlugin,
        ))
        .run();
}
