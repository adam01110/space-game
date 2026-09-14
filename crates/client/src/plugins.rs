use bevy::prelude::*;

use crate::{camera, input, network, player};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum ClientStartup {
    Camera,
    Connection,
}

pub(super) struct ClientAppPlugin;

impl Plugin for ClientAppPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Startup,
            (ClientStartup::Camera, ClientStartup::Connection).chain(),
        )
        .add_plugins((
            camera::ClientCameraPlugin,
            input::ClientInputPlugin,
            network::ClientNetworkPlugin,
            player::ClientPlayerPlugin,
        ));
    }
}
