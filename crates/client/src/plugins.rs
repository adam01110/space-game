use bevy::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_extended_ui::framework::ExtendedFrameworkConfiguration;
use bevy_extended_ui::{ExtendedCam, ExtendedUiConfiguration, ExtendedUiPlugin};
#[cfg(target_family = "wasm")]
use bevy_extended_ui::{html::HtmlSource, io::HtmlAsset, old::registry::UiRegistry};
use bevy_resvg::prelude::SvgPlugin;
#[cfg(feature = "dev")]
use lightyear::frame_interpolation::FrameInterpolationSystems;

#[cfg(not(target_family = "wasm"))]
use crate::menu;
#[cfg(target_family = "wasm")]
use crate::page::PagePlugin;
use crate::{
    abilities, background, beacons, camera, flame, frame, hud, input, network, player,
    remote_health,
};
#[cfg(feature = "dev")]
use crate::{arena, gizmos};

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
        // The framework's filesystem-based component discovery and its `assets/index.html`
        // entrypoint are native-only, so the browser client links the frame and HUD templates into
        // the entrypoint it registers itself. The page owns the menu, and the native build's menu
        // component is deliberately absent here.
        #[cfg(target_family = "wasm")]
        app.insert_resource(ExtendedUiConfiguration {
            camera: ExtendedCam::None,
            assets_path: "assets/ui/".into(),
            ..default()
        })
        .add_plugins((
            ExtendedUiPlugin,
            frame::NativeFramePlugin,
            hud::NativeHudPlugin,
            PagePlugin,
        ))
        .add_systems(Startup, load_browser_hud);
        app.add_plugins((
            remote_health::RemoteHealthPlugin,
            SvgPlugin,
            abilities::ClientAbilitiesPlugin,
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

// The framework's filesystem-based component discovery is native-only, so the browser client goes
// through the legacy registry instead. That registry is what fills the structure map's active list
// while the framework is absent, and the builder spawns no widget until that list names a key: a
// bare HtmlSource parses the HUD template and then never draws it.
#[cfg(target_family = "wasm")]
#[expect(
    deprecated,
    reason = "the registry is the only way to activate UI while the framework is absent"
)]
fn load_browser_hud(mut html_assets: ResMut<Assets<HtmlAsset>>, mut registry: ResMut<UiRegistry>) {
    let handle = html_assets.add(HtmlAsset {
        html: browser_hud_html(),
        stylesheets: Vec::new(),
    });

    // The registry name becomes the source key the parsed HUD is stored and built under.
    registry.add_and_use("browser-hud".to_owned(), HtmlSource::from_handle(handle));
}

// The frame first, as the background it is, then the readouts that sit on it: the build order of
// `assets/index.html`, whose component tags the framework resolves on native.
#[cfg(any(test, target_family = "wasm"))]
pub(super) fn browser_hud_html() -> String {
    format!(
        "<html><head><meta name=\"browser-hud\"><link rel=\"stylesheet\" href=\"ui/frame.css\"><link rel=\"stylesheet\" href=\"ui/hud.css\"></head><body><div id=\"hud-remote-health\"></div>{}{}</body></html>",
        include_str!("../../../assets/ui/frame.component.html"),
        include_str!("../../../assets/ui/hud.component.html")
    )
}
