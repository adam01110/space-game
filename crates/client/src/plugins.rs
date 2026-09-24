use bevy::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_extended_ui::framework::ExtendedFrameworkConfiguration;
use bevy_extended_ui::{ExtendedCam, ExtendedUiConfiguration, ExtendedUiPlugin};
#[cfg(target_family = "wasm")]
use bevy_extended_ui::{html::HtmlSource, io::HtmlAsset};
use bevy_resvg::prelude::SvgPlugin;
#[cfg(feature = "dev")]
use lightyear::frame_interpolation::FrameInterpolationSystems;

use crate::{
    abilities, asteroids, background, beacons, camera, flame, hud, input, network, player,
    remote_health,
};
#[cfg(feature = "dev")]
use crate::{arena, gizmos};
#[cfg(not(target_family = "wasm"))]
use crate::{frame, menu};

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
        // Physics gizmo rendering is a development-only diagnostic; overlays follow the
        // interpolated presentation, so they run after the frame interpolation pass.
        #[cfg(feature = "dev")]
        app.add_systems(
            PostUpdate,
            (
                arena::draw_arena,
                gizmos::draw_shot_hitboxes,
                gizmos::draw_beam_hitboxes,
            )
                .after(FrameInterpolationSystems::Interpolate),
        );
        #[cfg(not(target_family = "wasm"))]
        app.insert_resource(ExtendedUiConfiguration {
            camera: ExtendedCam::None,
            assets_path: "assets/ui/".into(),
            framework_components_path: "ui".into(),
            ..default()
        })
        .insert_resource(ExtendedFrameworkConfiguration {
            assets_component_root: "ui".into(),
            rust_component_root: format!("{}/src", env!("CARGO_MANIFEST_DIR")),
            asset_root_fs_path: format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")),
            index_html_file: "index.html".into(),
        })
        .add_plugins((
            ExtendedUiPlugin,
            menu::NativeMenuPlugin,
            frame::NativeFramePlugin,
            hud::NativeHudPlugin,
        ));
        #[cfg(target_family = "wasm")]
        app.insert_resource(ExtendedUiConfiguration {
            camera: ExtendedCam::None,
            assets_path: "assets/ui/".into(),
            ..default()
        })
        .add_plugins((ExtendedUiPlugin, hud::NativeHudPlugin))
        .add_systems(Startup, load_browser_hud);
        app.add_plugins((
            remote_health::RemoteHealthPlugin,
            SvgPlugin,
            abilities::ClientAbilitiesPlugin,
            asteroids::ClientAsteroidPlugin,
            background::ClientBackgroundPlugin,
            beacons::ClientBeaconPlugin,
            camera::ClientCameraPlugin,
            flame::ClientFlamePlugin,
            input::ClientInputPlugin,
            network::ClientNetworkPlugin,
            player::ClientPlayerPlugin,
        ));
    }
}

// The framework's filesystem-based component discovery is native-only. On wasm, load the same
// HUD template and stylesheet as extended-UI assets without loading the native menu or frame.
#[cfg(target_family = "wasm")]
fn load_browser_hud(mut commands: Commands, mut html_assets: ResMut<Assets<HtmlAsset>>) {
    let handle = html_assets.add(HtmlAsset {
        html: browser_hud_html(),
        stylesheets: Vec::new(),
    });
    commands.spawn(HtmlSource::from_handle(handle));
}

#[cfg(any(test, target_family = "wasm"))]
pub(super) fn browser_hud_html() -> String {
    format!(
        "<html><head><meta name=\"browser-hud\"><link rel=\"stylesheet\" href=\"ui/hud.css\"></head><body><div id=\"hud-remote-health\"></div>{}</body></html>",
        include_str!("../../../assets/ui/hud.component.html")
    )
}
