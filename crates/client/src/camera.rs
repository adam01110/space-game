use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::{PrimaryWindow, WindowResized},
};
use lightyear::prelude::input::native::InputMarker;

use project_protocol::{PlayerInput, PlayerPosition};

use super::plugins::ClientStartup;

pub(super) struct ClientCameraPlugin;

impl Plugin for ClientCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera.in_set(ClientStartup::Camera))
            .add_systems(PreUpdate, resize_canvas)
            .add_systems(Update, follow_player);
    }
}

/// Size of one rendered pixel in world and window units.
pub(super) const PIXEL_SIZE: f32 = 2.0;

/// How quickly the camera approaches the player position.
const CAMERA_DECAY_RATE: f32 = 2.0;
const GAMEPLAY_LAYERS: RenderLayers = RenderLayers::layer(0);
const CANVAS_LAYERS: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
pub(super) struct GameplayCamera;

#[derive(Component)]
struct GameplayCanvas;

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
        Msaa::Sample2,
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
    players: Query<&PlayerPosition, With<InputMarker<PlayerInput>>>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, With<GameplayCamera>)>,
    time: Res<Time>,
) {
    let (Ok(position), Ok(mut camera_transform)) = (players.single(), cameras.single_mut()) else {
        return;
    };

    let target = Vec3::new(position.x, position.y, camera_transform.translation.z);
    camera_transform
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
}
