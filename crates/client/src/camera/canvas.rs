use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::{PrimaryWindow, WindowResized},
};

use super::{GameplayCamera, PIXEL_SIZE};

pub(super) fn canvas_size(window_width: f32, window_height: f32) -> Extent3d {
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

// The low-resolution render target that gameplay is drawn into.
pub(super) fn create_canvas(images: &mut Assets<Image>, size: Extent3d) -> Handle<Image> {
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("gameplay_canvas"),
            size,
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
    canvas.resize(size);
    images.add(canvas)
}

pub(super) fn resize_canvas(
    mut resize_messages: MessageReader<WindowResized>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    gameplay_camera: Single<&RenderTarget, With<GameplayCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    // The render target is fixed for the run, so it is resolved once instead of per message.
    let RenderTarget::Image(canvas) = &*gameplay_camera else {
        return;
    };

    for resized in resize_messages.read() {
        if resized.window != *primary_window {
            continue;
        }

        if let Some(mut image) = images.get_mut(&canvas.handle) {
            image.resize(canvas_size(resized.width, resized.height));
        }
    }
}
