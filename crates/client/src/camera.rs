mod canvas;

use avian2d::prelude::PhysicsSystems;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    transform::TransformSystems,
    window::PrimaryWindow,
};
use lightyear::prelude::input::native::InputMarker;

use project_protocol::PlayerInput;

use super::plugins::ClientStartup;

use canvas::{canvas_size, create_canvas, resize_canvas};

pub(super) struct ClientCameraPlugin;

impl Plugin for ClientCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera.in_set(ClientStartup::Camera))
            .add_systems(PreUpdate, resize_canvas)
            .add_systems(
                PostUpdate,
                follow_player
                    .after(PhysicsSystems::Writeback)
                    .before(TransformSystems::Propagate),
            );
    }
}

// Size of one rendered pixel in world and window units.
pub(super) const PIXEL_SIZE: f32 = 4.0;

// How quickly the camera approaches the player position.
const CAMERA_DECAY_RATE: f32 = 6.0;
const GAMEPLAY_LAYERS: RenderLayers = RenderLayers::layer(0);
const CANVAS_LAYERS: RenderLayers = RenderLayers::layer(1);
#[cfg(feature = "dev")]
pub(super) const DEBUG_RENDER_LAYERS: RenderLayers = RenderLayers::layer(2);

#[derive(Component)]
pub(super) struct GameplayCamera;

#[derive(Component)]
struct GameplayCanvas;

#[derive(Component)]
struct DebugCamera;

type GameplayCameraFilter = (With<Camera2d>, With<GameplayCamera>, Without<DebugCamera>);
type DebugCameraFilter = (
    With<Camera2d>,
    With<DebugCamera>,
    Without<GameplayCamera>,
    Without<InputMarker<PlayerInput>>,
);

fn setup_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let size = canvas_size(window.width(), window.height());
    let canvas = create_canvas(&mut images, size);

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: PIXEL_SIZE,
            ..OrthographicProjection::default_2d()
        }),
        RenderTarget::Image(canvas.clone().into()),
        Msaa::Off,
        GameplayCamera,
        GAMEPLAY_LAYERS,
    ));

    commands.spawn((Sprite::from_image(canvas), GameplayCanvas, CANVAS_LAYERS));

    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0 / PIXEL_SIZE,
            ..OrthographicProjection::default_2d()
        }),
        Msaa::Off,
        CANVAS_LAYERS,
    ));

    // Draw diagnostics directly to the window instead of baking them into the
    // low-resolution pixel-art canvas. This preserves smooth subpixel lines.
    // Only the physics debug build renders this extra camera pass.
    #[cfg(feature = "dev")]
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // All cameras targeting the window must use the same sample count. Native
        // resolution still avoids the 4x pixelation from the gameplay canvas.
        Msaa::Off,
        DebugCamera,
        DEBUG_RENDER_LAYERS,
    ));
}

fn follow_player(
    // InputMarker<PlayerInput> is attached only to the player receiving input from this client.
    players: Query<&Transform, (With<InputMarker<PlayerInput>>, Without<GameplayCamera>)>,
    mut gameplay_cameras: Query<&mut Transform, GameplayCameraFilter>,
    mut debug_cameras: Query<&mut Transform, DebugCameraFilter>,
    time: Res<Time>,
) {
    let (Ok(player_transform), Ok(mut camera_transform)) =
        (players.single(), gameplay_cameras.single_mut())
    else {
        return;
    };

    let target = Vec3::new(
        player_transform.translation.x,
        player_transform.translation.y,
        camera_transform.translation.z,
    );
    camera_transform
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());

    if let Ok(mut debug_transform) = debug_cameras.single_mut() {
        debug_transform.translation = camera_transform.translation;
    }
}
