mod camera;
mod guest;
mod input;
mod network;
mod player;

use bevy::prelude::*;
use lightyear::prelude::{client::input::InputSystems, client::*};

use project_game::{ClientSimulationPlugin, GamePlugin};
use project_protocol::ProtocolPlugin;

use crate::{
    camera::{follow_player, setup_camera},
    input::buffer_player_input,
    network::{setup_connection, update_connection},
    player::{
        add_interpolated_player_visual, add_predicted_player_visual, prepare_controlled_player,
    },
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ClientPlugins::default(),
            ProtocolPlugin,
            GamePlugin,
            ClientSimulationPlugin,
        ))
        .add_systems(Startup, (setup_camera, setup_connection))
        .add_systems(
            FixedPreUpdate,
            buffer_player_input.in_set(InputSystems::WriteClientInputs),
        )
        .add_systems(Update, (follow_player, update_connection))
        .add_observer(prepare_controlled_player)
        .add_observer(add_predicted_player_visual)
        .add_observer(add_interpolated_player_visual)
        .run();
}
