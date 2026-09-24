// The headless camera integration test initializes wgpu's ManualTextureViews resource.
#![cfg_attr(test, recursion_limit = "256")]

mod abilities;
mod arena;
mod background;
mod beacons;
mod blasters;
mod camera;
mod flame;
mod frame;
#[cfg(not(target_family = "wasm"))]
#[path = "frame.component.rs"]
mod frame_component;
mod gizmos;
mod guest;
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
#[cfg(target_family = "wasm")]
mod page;
mod palette;
mod phase_beam;
mod player;
mod plugins;
mod remote_health;

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

use space_game_game::{ClientSimulationPlugin, GamePlugin, SERVER_UPS};
use space_game_protocol::ProtocolPlugin;

#[cfg(feature = "dev")]
use crate::camera::DEBUG_RENDER_LAYERS;
use crate::plugins::ClientAppPlugin;

// Bevy resolves assets from `CARGO_MANIFEST_DIR`, which cargo sets to this crate, so the
// workspace-level assets directory must be addressed from there. The browser build serves assets
// from the web root and keeps the default path.
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
                    // VSync bounds rendering to the display refresh rate instead of spinning
                    // the GPU and CPU at an unbounded frame rate.
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
    app.run();
}
