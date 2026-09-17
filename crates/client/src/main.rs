mod abilities;
mod arena;
mod blasters;
mod camera;
mod confirmed;
mod focus;
mod guest;
mod input;
mod network;
mod phase_beam;
mod player;
mod plugins;

#[cfg(test)]
mod tests;

use std::time::Duration;

#[cfg(feature = "dev")]
use avian2d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
#[cfg(not(feature = "dev"))]
use bevy::winit::WinitSettings;
use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use lightyear::prelude::client::*;

use space_game_game::{ClientSimulationPlugin, GamePlugin, SERVER_UPS};
use space_game_protocol::ProtocolPlugin;

#[cfg(feature = "dev")]
use crate::camera::DEBUG_RENDER_LAYERS;
use crate::plugins::ClientAppPlugin;

fn main() {
    let mut app = App::new();
    #[cfg(feature = "dev")]
    let app = app
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
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
        .add_plugins(PhysicsDebugPlugin);
    #[cfg(not(feature = "dev"))]
    let app = app
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // VSync bounds rendering to the display refresh rate instead of
                        // spinning the GPU and CPU at an unbounded frame rate.
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(WinitSettings {
            // Reduced update cadence while unfocused; the focus plugin resumes simulation
            // from authoritative state when focus returns.
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f64(
                1.0 / 30.0,
            )),
            ..default()
        });

    app.add_plugins((
        ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
        },
        ProtocolPlugin,
        GamePlugin,
        ClientSimulationPlugin,
        ClientAppPlugin,
    ))
    .run();
}
