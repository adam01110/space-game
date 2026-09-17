use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::prelude::{
    LocalTimelineSync, ReplicationCheckpointMap,
    input::native::{ActionState, InputMarker},
};

use space_game_protocol::PlayerInput;

use crate::{
    focus::{ClientFocusPlugin, FocusPrediction},
    input::{AbilityInputs, buffer_player_input, capture_ability_inputs},
};

#[test]
fn suspension_sends_neutral_controls_without_resetting_counters() {
    let mut app = App::new();
    app.add_plugins(ClientFocusPlugin)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<AbilityInputs>()
        .init_resource::<LocalTimelineSync>()
        .init_resource::<ReplicationCheckpointMap>()
        .add_systems(PreUpdate, capture_ability_inputs)
        .add_systems(Update, buffer_player_input);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let player = app
        .world_mut()
        .spawn((
            GlobalTransform::default(),
            ActionState::<PlayerInput>::default(),
            InputMarker::<PlayerInput>::default(),
        ))
        .id();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::Space);
        keys.press(KeyCode::KeyR);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(Update);
    let active = app
        .world()
        .get::<ActionState<PlayerInput>>(player)
        .expect("input")
        .0
        .clone();
    assert_eq!(active.movement, Vec2::Y);
    assert!(active.phase_beam);
    assert_eq!(active.blaster_clicks, 1);
    assert_eq!(active.blaster_reload_requests, 1);

    app.world_mut()
        .get_mut::<Window>(window)
        .expect("window")
        .focused = false;
    // Even stale pressed/just-pressed events must not become background controls.
    for _ in 0..5 {
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_schedule(Update);
        let neutral = &app
            .world()
            .get::<ActionState<PlayerInput>>(player)
            .expect("input")
            .0;
        assert_eq!(neutral.movement, Vec2::ZERO);
        assert!(!neutral.phase_beam);
        assert_eq!(neutral.aim, active.aim);
        assert_eq!(neutral.blaster_clicks, active.blaster_clicks);
        assert_eq!(
            neutral.blaster_reload_requests,
            active.blaster_reload_requests
        );
    }
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("window")
        .focused = true;
    app.world_mut().run_schedule(PreUpdate);
    assert!(!app.world().resource::<FocusPrediction>().accepts_input());
    // Simulate successful authoritative restoration; held keys are still valid,
    // but consumed mouse/reload edges must not be synthesized on reactivation.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.insert_resource(FocusPrediction::default());
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(Update);
    let resumed = &app
        .world()
        .get::<ActionState<PlayerInput>>(player)
        .expect("input")
        .0;
    assert_eq!(resumed.movement, Vec2::Y);
    assert_eq!(resumed.blaster_clicks, active.blaster_clicks);
    assert_eq!(
        resumed.blaster_reload_requests,
        active.blaster_reload_requests
    );
}
