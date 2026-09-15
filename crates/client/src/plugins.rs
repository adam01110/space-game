use bevy::prelude::*;
use lightyear::frame_interpolation::FrameInterpolationSystems;

use crate::{abilities, arena, camera, input, network, player};

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
        .add_systems(
            PostUpdate,
            arena::draw_arena.after(FrameInterpolationSystems::Interpolate),
        )
        .add_plugins((
            abilities::ClientAbilitiesPlugin,
            camera::ClientCameraPlugin,
            input::ClientInputPlugin,
            network::ClientNetworkPlugin,
            player::ClientPlayerPlugin,
        ));
    }
}
