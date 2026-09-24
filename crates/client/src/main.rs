mod abilities;
mod arena;
mod background;
mod beacons;
mod blasters;
mod camera;
#[cfg(not(target_family = "wasm"))]
mod frame;
#[cfg(not(target_family = "wasm"))]
#[path = "frame.component.rs"]
mod frame_component;
mod gizmos;
mod guest;
#[cfg(not(target_family = "wasm"))]
mod hud;
#[cfg(not(target_family = "wasm"))]
#[path = "hud.component.rs"]
mod hud_component;
mod input;
#[cfg(not(target_family = "wasm"))]
mod menu;
#[cfg(not(target_family = "wasm"))]
#[path = "menu.component.rs"]
mod menu_component;
mod network;
mod palette;
mod phase_beam;
mod player;
mod plugins;

#[cfg(test)]
mod tests;

use std::time::Duration;

#[cfg(feature = "dev")]
use avian2d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use lightyear::prelude::client::*;
#[cfg(target_family = "wasm")]
use lightyear::prelude::input::native::InputMarker;

use space_game_game::{ClientSimulationPlugin, GamePlugin, SERVER_UPS};
#[cfg(target_family = "wasm")]
use space_game_protocol::PlayerInput;
use space_game_protocol::ProtocolPlugin;

#[cfg(feature = "dev")]
use crate::camera::DEBUG_RENDER_LAYERS;
use crate::plugins::ClientAppPlugin;

// Bevy resolves assets from `CARGO_MANIFEST_DIR`, which cargo sets to this crate, so the
// workspace-level assets directory has to be addressed from there. The browser build serves
// assets from the web root and keeps the default path.
#[cfg(not(target_family = "wasm"))]
const ASSET_PATH: &str = "../../assets";
#[cfg(target_family = "wasm")]
const ASSET_PATH: &str = "assets";

fn main() {
    let mut app = App::new();
    #[cfg(feature = "dev")]
    let app = app
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: ASSET_PATH.to_owned(),
                    ..default()
                })
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
    let app = app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: ASSET_PATH.to_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    // VSync bounds rendering to the display refresh rate instead of
                    // spinning the GPU and CPU at an unbounded frame rate.
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
    );

    app.add_plugins((
        ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
        },
        ProtocolPlugin,
        GamePlugin,
        ClientSimulationPlugin,
        ClientAppPlugin,
    ));
    #[cfg(target_family = "wasm")]
    app.add_systems(Update, signal_game_ready);
    app.run();
}

#[cfg(target_family = "wasm")]
fn signal_game_ready(
    players: Query<(), With<InputMarker<PlayerInput>>>,
    mut signaled: Local<bool>,
) {
    if *signaled || players.is_empty() {
        return;
    } else if web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("game-shell"))
        .is_some_and(|shell| shell.set_attribute("data-game-ready", "true").is_ok())
    {
        *signaled = true;
    }
}
