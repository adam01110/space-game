mod controls;

use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::{
    input::{input_message::InputMessage, native::prelude::NativeStateSequence},
    prelude::{
        LocalTimelineSync, MessageReceiver, MessageSystems,
        client::input::InputSystems,
        input::native::{ActionState, InputMarker},
    },
};

use space_game_protocol::PlayerInput;

use super::camera::GameplayCamera;

use controls::{aim_direction, axis};

pub(super) struct ClientInputPlugin;

impl Plugin for ClientInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AbilityInputs>()
            .init_resource::<SessionInputReset>();
        app.add_systems(
            PreUpdate,
            (finish_session_input_reset, capture_ability_inputs)
                .chain()
                .after(bevy::input::InputSystems),
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
// end-of-frame cleanup warns about every unread message. Those packets cannot be retained across
// frames and later ones carry redundant state, so discard them during startup.
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

// Use the last rendered player and camera poses together: Physics Position has already been
// restored to the current tick here and would mix timelines during catch-up ticks.
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
struct AbilityInputs {
    blaster_clicks: u8,
    blaster_reload_requests: u8,
    phase_beam: bool,
    boost: bool,
}

#[derive(Resource, Default)]
struct SessionInputReset(bool);

// Run during retirement, before packet receive. Also clear events that accumulated during the
// pause; otherwise an old key-down could come back as held input.
pub(super) fn reset_session_inputs(world: &mut World) {
    world.insert_resource(AbilityInputs::default());
    world.insert_resource(SessionInputReset(true));
    world
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardInput>>()
        .clear();
    world
        .resource_mut::<Messages<bevy::input::mouse::MouseButtonInput>>()
        .clear();
    world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
    world.resource_mut::<ButtonInput<MouseButton>>().reset_all();
}

// OS input can arrive after First, so clear again after Bevy processes it, before capturing
// abilities. Ordinary frames must NOT reset the captured click counters.
fn finish_session_input_reset(
    mut reset: ResMut<SessionInputReset>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
) {
    if std::mem::take(&mut reset.0) {
        keyboard.reset_all();
        mouse.reset_all();
    }
}

// Capture ability controls once per render frame. Counters preserve discrete presses across zero
// or multiple fixed ticks, while the beam keeps its current held state.
fn capture_ability_inputs(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    players: Query<(), With<InputMarker<PlayerInput>>>,
    mut inputs: ResMut<AbilityInputs>,
) {
    if players.is_empty() {
        *inputs = AbilityInputs::default();
        return;
    }

    inputs.blaster_clicks = inputs
        .blaster_clicks
        .wrapping_add(u8::from(mouse.just_pressed(MouseButton::Left)));

    inputs.blaster_reload_requests = inputs
        .blaster_reload_requests
        .wrapping_add(u8::from(keyboard.just_pressed(KeyCode::KeyR)));

    inputs.phase_beam = mouse.pressed(MouseButton::Right);
    inputs.boost = keyboard.pressed(KeyCode::Space);
}

fn buffer_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    abilities: Res<AbilityInputs>,
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
        blaster_clicks: abilities.blaster_clicks,
        blaster_reload_requests: abilities.blaster_reload_requests,
        phase_beam: abilities.phase_beam,
        boost: abilities.boost,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_beam_tracks_right_mouse_button_not_space() {
        let mut app = App::new();
        app.init_resource::<AbilityInputs>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, capture_ability_inputs);
        app.world_mut().spawn(InputMarker::<PlayerInput>::default());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);

        app.update();
        assert!(!app.world().resource::<AbilityInputs>().phase_beam);
        assert!(app.world().resource::<AbilityInputs>().boost);

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        app.update();
        assert!(app.world().resource::<AbilityInputs>().phase_beam);

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Right);
        app.update();
        assert!(!app.world().resource::<AbilityInputs>().phase_beam);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Space);
        app.update();
        assert!(!app.world().resource::<AbilityInputs>().boost);
    }
}
