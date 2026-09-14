use avian2d::prelude::PhysicsSystems;
use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    transform::TransformSystems,
    window::{PrimaryWindow, WindowResized},
};
use lightyear::prelude::input::native::InputMarker;

use project_protocol::PlayerInput;

use super::plugins::ClientStartup;

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

/// Size of one rendered pixel in world and window units.
pub(super) const PIXEL_SIZE: f32 = 4.0;

/// How quickly the camera approaches the player position.
const CAMERA_DECAY_RATE: f32 = 6.0;
const GAMEPLAY_LAYERS: RenderLayers = RenderLayers::layer(0);
const CANVAS_LAYERS: RenderLayers = RenderLayers::layer(1);
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
    let canvas_size = canvas_size(window.width(), window.height());
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("gameplay_canvas"),
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    canvas.resize(canvas_size);
    let canvas = images.add(canvas);

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

fn resize_canvas(
    mut resize_messages: MessageReader<WindowResized>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    gameplay_camera: Single<&RenderTarget, With<GameplayCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    for resized in resize_messages.read() {
        if resized.window != *primary_window {
            continue;
        }

        let RenderTarget::Image(canvas) = &*gameplay_camera else {
            return;
        };

        if let Some(mut image) = images.get_mut(&canvas.handle) {
            image.resize(canvas_size(resized.width, resized.height));
        }
    }
}

fn canvas_size(window_width: f32, window_height: f32) -> Extent3d {
    let size = (Vec2::new(window_width, window_height) / PIXEL_SIZE)
        .ceil()
        .as_uvec2()
        .max(UVec2::ONE);
    Extent3d {
        width: size.x,
        height: size.y,
        ..default()
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn camera_smoothly_follows_visual_writeback_before_global_propagation() {
        let mut app = App::new();
        app.add_plugins((bevy::transform::TransformPlugin, ClientCameraPlugin));
        app.insert_resource(Time::<()>::default());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0 / 60.0));

        let player = app
            .world_mut()
            .spawn((Transform::default(), InputMarker::<PlayerInput>::default()))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Camera2d,
                GameplayCamera,
                Transform::from_xyz(0.0, 0.0, 99.0),
            ))
            .id();
        app.add_systems(
            PostUpdate,
            (move |mut players: Query<&mut Transform, With<InputMarker<PlayerInput>>>| {
                players.single_mut().unwrap().translation = Vec3::new(37.0, -12.0, 0.0);
            })
            .in_set(PhysicsSystems::Writeback),
        );

        app.world_mut().run_schedule(PostUpdate);

        let camera_global = app
            .world()
            .get::<GlobalTransform>(camera)
            .unwrap()
            .translation();
        let player_global = app
            .world()
            .get::<GlobalTransform>(player)
            .unwrap()
            .translation();
        assert!(camera_global.x > 0.0 && camera_global.x < player_global.x);
        assert!(camera_global.y < 0.0 && camera_global.y > player_global.y);
        assert_eq!(camera_global.z, 99.0);
        assert_eq!(player_global, Vec3::new(37.0, -12.0, 0.0));
    }
}
