use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::prelude::input::native::{ActionState, InputMarker};
use project_protocol::{PlayerInput, PlayerPosition};

pub(super) fn buffer_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut players: Query<
        (&PlayerPosition, &mut ActionState<PlayerInput>),
        With<InputMarker<PlayerInput>>,
    >,
) {
    let Ok((position, mut action_state)) = players.single_mut() else {
        return;
    };

    let horizontal = axis(&keyboard, KeyCode::KeyA, KeyCode::KeyD);
    let vertical = axis(&keyboard, KeyCode::KeyS, KeyCode::KeyW);
    let aim = windows
        .single()
        .ok()
        .zip(cameras.single().ok())
        .and_then(|(window, (camera, camera_transform))| {
            let cursor_position = window.cursor_position()?;
            let cursor_world = camera
                .viewport_to_world_2d(camera_transform, cursor_position)
                .ok()?;
            (cursor_world - position.0).try_normalize()
        })
        .unwrap_or(action_state.0.aim);

    action_state.0 = PlayerInput {
        movement: Vec2::new(horizontal, vertical),
        aim,
    };
}

fn axis(keyboard: &ButtonInput<KeyCode>, negative: KeyCode, positive: KeyCode) -> f32 {
    let positive = if keyboard.pressed(positive) { 1.0 } else { 0.0 };
    let negative = if keyboard.pressed(negative) { 1.0 } else { 0.0 };

    positive - negative
}
