mod abilities;
mod arena;
mod blasters;
mod camera;
mod guest;
mod input;
mod network;
mod phase_beam;
mod player;
mod plugins;

use std::time::Duration;

use avian2d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use lightyear::prelude::client::*;

use project_game::{ClientSimulationPlugin, GamePlugin, SERVER_UPS};
use project_protocol::ProtocolPlugin;

use crate::{camera::DEBUG_RENDER_LAYERS, plugins::ClientAppPlugin};

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
            PhysicsDebugPlugin,
            ClientAppPlugin,
        ))
        .insert_gizmo_config(
            PhysicsGizmos {
                // Avoid brightness changes as contact bodies sleep and wake.
                sleeping_color_multiplier: None,
                ..default()
            },
            GizmoConfig {
                line: GizmoLineConfig {
                    width: 2.0,
                    ..default()
                },
                render_layers: DEBUG_RENDER_LAYERS,
                ..default()
            },
        )
        .run();
}
