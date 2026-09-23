mod canvas;

use avian2d::prelude::PhysicsSystems;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    transform::TransformSystems,
    window::PrimaryWindow,
};
use lightyear::prelude::input::native::InputMarker;

use space_game_protocol::PlayerInput;

use super::plugins::ClientStartup;

use canvas::{canvas_size, create_canvas, resize_canvas};

pub(super) struct ClientCameraPlugin;

// Camera placement, so that presentation that reads the camera can run after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum CameraSystems {
    Follow,
}

impl Plugin for ClientCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera.in_set(ClientStartup::Camera))
            .add_systems(PreUpdate, resize_canvas)
            .add_systems(
                PostUpdate,
                follow_player
                    .in_set(CameraSystems::Follow)
                    .after(PhysicsSystems::Writeback)
                    .before(TransformSystems::Propagate),
            );
        #[cfg(feature = "dev")]
        app.init_resource::<DebugZoom>()
            .add_systems(Update, toggle_debug_zoom);
    }
}

// Size of one rendered pixel in world and window units.
pub(super) const PIXEL_SIZE: f32 = 4.0;

// Scale of the camera that displays the pixel-art canvas on the window. The canvas holds the
// gameplay view at one pixel per `PIXEL_SIZE` world units, so this undoes that downscale.
pub(super) const CANVAS_CAMERA_SCALE: f32 = 1.0 / PIXEL_SIZE;

// How much further the debug keybind pulls the view back. Five times the world on screen is enough
// for the edge of the streamed backdrop to sit well inside the window.
#[cfg(feature = "dev")]
pub(super) const DEBUG_ZOOM_FACTOR: f32 = 2.0;

// How quickly the camera approaches the player position.
const CAMERA_DECAY_RATE: f32 = 6.0;
const GAMEPLAY_LAYERS: RenderLayers = RenderLayers::layer(0);
const CANVAS_LAYERS: RenderLayers = RenderLayers::layer(1);
#[cfg(feature = "dev")]
pub(super) const DEBUG_RENDER_LAYERS: RenderLayers = RenderLayers::layer(2);

#[derive(Component)]
pub(super) struct GameplayCamera;

// The rendered gameplay canvas, and the camera that displays it on the window.
#[derive(Component)]
struct GameplayCanvas;

#[derive(Component)]
pub(super) struct CanvasCamera;

#[derive(Component)]
pub(super) struct DebugCamera;

// Whether the debug zoom keybind is holding the view back. Development builds only.
#[cfg(feature = "dev")]
#[derive(Resource, Default)]
pub(super) struct DebugZoom(bool);

type GameplayCameraFilter = (With<Camera2d>, With<GameplayCamera>, Without<DebugCamera>);
// Only the debug zoom moves the canvas camera, so its filter exists with the keybind.
#[cfg(feature = "dev")]
type CanvasCameraFilter = (
    With<Camera2d>,
    With<CanvasCamera>,
    Without<GameplayCamera>,
    Without<DebugCamera>,
);
type DebugCameraFilter = (
    With<Camera2d>,
    With<DebugCamera>,
    Without<GameplayCamera>,
    Without<CanvasCamera>,
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
            scale: CANVAS_CAMERA_SCALE,
            ..OrthographicProjection::default_2d()
        }),
        Msaa::Off,
        CanvasCamera,
        CANVAS_LAYERS,
        #[cfg(not(target_family = "wasm"))]
        IsDefaultUiCamera,
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
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
        // All cameras targeting the window must use the same sample count. Native
        // resolution still avoids the 4x pixelation from the gameplay canvas.
        Msaa::Off,
        DebugCamera,
        DEBUG_RENDER_LAYERS,
    ));
}

// Debug keybind: `Z` pulls the view back by `DEBUG_ZOOM_FACTOR`, so the game view shrinks to a
// fifth of the window with the streamed backdrop still visible around it.
//
// The one camera the zoom deliberately leaves alone is the gameplay camera: the backdrop streamer
// reads its projection as the view it has to cover, so zooming it out would stream several times
// the chunks and push the edge of what is spawned off screen, which is the one thing this keybind
// is for looking at. Shrinking what displays the canvas instead keeps the whole streamed region
// inside the window around the smaller view of the game.
//
// The scale is applied every frame from the toggle, so a camera spawned after the key was pressed
// is zoomed too.
#[cfg(feature = "dev")]
pub(super) fn toggle_debug_zoom(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut zoom: ResMut<DebugZoom>,
    mut canvas_cameras: Query<&mut Projection, CanvasCameraFilter>,
    mut debug_cameras: Query<&mut Projection, DebugCameraFilter>,
) {
    if keyboard.just_pressed(KeyCode::KeyZ) {
        zoom.0 = !zoom.0;
        info!(
            "Debug zoom {} ({}x smaller)",
            if zoom.0 { "on" } else { "off" },
            DEBUG_ZOOM_FACTOR
        );
    }

    let factor = if zoom.0 { DEBUG_ZOOM_FACTOR } else { 1.0 };

    for mut projection in &mut canvas_cameras {
        set_scale(&mut projection, CANVAS_CAMERA_SCALE * factor);
    }

    // Diagnostics are drawn to the window rather than through the canvas, so they zoom on the same
    // factor with their own scale to stay aligned with the canvas they annotate.
    for mut projection in &mut debug_cameras {
        set_scale(&mut projection, 1.0 * factor);
    }
}

// Only the orthographic canvas and diagnostic views are zoomable; any other projection is left as
// it was spawned.
#[cfg(feature = "dev")]
const fn set_scale(projection: &mut Projection, scale: f32) {
    if let Projection::Orthographic(orthographic) = projection {
        orthographic.scale = scale;
    }
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
