use bevy::prelude::*;
#[cfg(feature = "dev")]
use lightyear::frame_interpolation::FrameInterpolationSystems;

#[cfg(feature = "dev")]
use crate::arena;
use crate::{abilities, camera, input, network, player};

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
        );
        // Physics gizmo rendering is a development-only diagnostic.
        #[cfg(feature = "dev")]
        app.add_systems(
            PostUpdate,
            arena::draw_arena.after(FrameInterpolationSystems::Interpolate),
        );
        app.add_plugins((
            abilities::ClientAbilitiesPlugin,
            camera::ClientCameraPlugin,
            input::ClientInputPlugin,
            network::ClientNetworkPlugin,
            player::ClientPlayerPlugin,
        ));
    }
}
