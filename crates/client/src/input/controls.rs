use bevy::prelude::*;

use crate::camera::PIXEL_SIZE;

pub(super) fn aim_direction(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    player_position: Vec2,
) -> Option<Vec2> {
    let cursor_position = window.cursor_position()?;

    let canvas_size = camera.logical_viewport_size()?;
    let canvas_cursor = canvas_size / 2.0 + (cursor_position - window.size() / 2.0) / PIXEL_SIZE;

    let cursor_world = camera
        .viewport_to_world_2d(camera_transform, canvas_cursor)
        .ok()?;

    (cursor_world - player_position).try_normalize()
}

pub(super) fn axis(keyboard: &ButtonInput<KeyCode>, negative: KeyCode, positive: KeyCode) -> f32 {
    let positive = if keyboard.pressed(positive) { 1.0 } else { 0.0 };
    let negative = if keyboard.pressed(negative) { 1.0 } else { 0.0 };

    positive - negative
}
