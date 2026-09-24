use bevy::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_extended_ui::{
    ExtendedCam, ExtendedUiConfiguration, ExtendedUiPlugin,
    framework::ExtendedFrameworkConfiguration,
};
use bevy_resvg::prelude::SvgPlugin;
#[cfg(feature = "dev")]
use lightyear::frame_interpolation::FrameInterpolationSystems;

#[cfg(feature = "dev")]
use crate::arena;
use crate::{abilities, background, beacons, camera, input, network, player};
#[cfg(not(target_family = "wasm"))]
use crate::{frame, hud, menu};

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
        app.add_plugins((
            SvgPlugin,
            abilities::ClientAbilitiesPlugin,
            background::ClientBackgroundPlugin,
            beacons::ClientBeaconPlugin,
            camera::ClientCameraPlugin,
            input::ClientInputPlugin,
            network::ClientNetworkPlugin,
            player::ClientPlayerPlugin,
        ));
    }
}
