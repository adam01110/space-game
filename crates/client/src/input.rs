use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::prelude::input::native::{ActionState, InputMarker};

use project_protocol::{PlayerInput, PlayerPosition};

use crate::camera::GameplayCamera;

type PlayerInputQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerPosition,
        &'static mut ActionState<PlayerInput>,
    ),
    With<InputMarker<PlayerInput>>,
>;

pub(super) fn buffer_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<GameplayCamera>>,
    mut players: PlayerInputQuery,
) {
    let Ok((position, mut action_state)) = players.single_mut() else {
        return;
    };

    let horizontal = axis(&keyboard, KeyCode::KeyA, KeyCode::KeyD);
    let vertical = axis(&keyboard, KeyCode::KeyS, KeyCode::KeyW);

    let aim = match (windows.single(), cameras.single()) {
        (Ok(window), Ok((camera, camera_transform))) => {
            aim_direction(window, camera, camera_transform, position.0)
                .unwrap_or(action_state.0.aim)
        }
        _ => action_state.0.aim,
    };

    action_state.0 = PlayerInput {
        movement: Vec2::new(horizontal, vertical),
        aim,
    };
}

fn aim_direction(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    player_position: Vec2,
) -> Option<Vec2> {
    let cursor_position = window.cursor_position()?;
    let cursor_world = camera
        .viewport_to_world_2d(camera_transform, cursor_position)
        .ok()?;

    (cursor_world - player_position).try_normalize()
}

fn axis(keyboard: &ButtonInput<KeyCode>, negative: KeyCode, positive: KeyCode) -> f32 {
    let positive = if keyboard.pressed(positive) { 1.0 } else { 0.0 };
    let negative = if keyboard.pressed(negative) { 1.0 } else { 0.0 };

    positive - negative
}
