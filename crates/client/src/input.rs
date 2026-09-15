use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::{
    input::{input_message::InputMessage, native::prelude::NativeStateSequence},
    prelude::{
        client::input::InputSystems,
        input::native::{ActionState, InputMarker},
        LocalTimelineSync, MessageReceiver, MessageSystems,
    },
};

use project_protocol::PlayerInput;

use super::camera::{GameplayCamera, PIXEL_SIZE};

pub(super) struct ClientInputPlugin;

impl Plugin for ClientInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlasterClicks>();
        app.add_systems(
            PreUpdate,
            capture_blaster_clicks.after(bevy::input::InputSystems),
        );
        app.add_systems(
            PreUpdate,
            discard_remote_inputs_before_sync
                .after(MessageSystems::Receive)
                .before(InputSystems::ReceiveInputMessages),
        )
        .add_systems(
            FixedPreUpdate,
            buffer_player_input.in_set(InputSystems::WriteClientInputs),
        );
    }
}

// Lightyear cannot process rebroadcast inputs until the local timeline is synchronized, but its
// end-of-frame cleanup warns about every unread message. These packets cannot be retained across
// frames and later packets contain redundant state, so explicitly discard them during startup.
fn discard_remote_inputs_before_sync(
    timeline_sync: Res<LocalTimelineSync>,
    mut receivers: Query<&mut MessageReceiver<InputMessage<NativeStateSequence<PlayerInput>>>>,
) {
    if timeline_sync.is_synced() {
        return;
    }

    for mut receiver in &mut receivers {
        receiver.receive().for_each(drop);
    }
}

// Use the last rendered player and camera poses together. Physics Position has already
// been restored to the current tick here and would mix timelines during catch-up ticks.
type PlayerInputQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static GlobalTransform,
        &'static mut ActionState<PlayerInput>,
    ),
    With<InputMarker<PlayerInput>>,
>;

#[derive(Resource, Default)]
struct BlasterClicks(u32);

// Capture once per render frame, even when there are zero or multiple fixed ticks.
fn capture_blaster_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    players: Query<(), With<InputMarker<PlayerInput>>>,
    mut clicks: ResMut<BlasterClicks>,
) {
    if players.is_empty() {
        clicks.0 = 0;
    } else if mouse.just_pressed(MouseButton::Left) {
        clicks.0 = clicks.0.wrapping_add(1);
    }
}

fn buffer_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    clicks: Res<BlasterClicks>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<GameplayCamera>>,
    mut players: PlayerInputQuery,
) {
    let Ok((player_transform, mut action_state)) = players.single_mut() else {
        return;
    };

    let horizontal = axis(&keyboard, KeyCode::KeyA, KeyCode::KeyD);
    let vertical = axis(&keyboard, KeyCode::KeyS, KeyCode::KeyW);

    let aim = match (windows.single(), cameras.single()) {
        (Ok(window), Ok((camera, camera_transform))) => aim_direction(
            window,
            camera,
            camera_transform,
            player_transform.translation().truncate(),
        )
        .unwrap_or(action_state.0.aim),
        _ => action_state.0.aim,
    };

    action_state.0 = PlayerInput {
        movement: Vec2::new(horizontal, vertical),
        aim,
        blaster_clicks: clicks.0,
    };
}

fn aim_direction(
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

fn axis(keyboard: &ButtonInput<KeyCode>, negative: KeyCode, positive: KeyCode) -> f32 {
    let positive = if keyboard.pressed(positive) { 1.0 } else { 0.0 };
    let negative = if keyboard.pressed(negative) { 1.0 } else { 0.0 };

    positive - negative
}
