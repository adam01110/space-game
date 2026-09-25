#[cfg(not(target_family = "wasm"))]
#[path = "network/death.rs"]
mod death_tests;
#[path = "network/policy.rs"]
mod policy_tests;
#[path = "network/recovery.rs"]
mod recovery_tests;
#[cfg(not(target_family = "wasm"))]
mod wire;

use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use lightyear::{
    prediction::manager::StateRollbackMetadata,
    prelude::{client::*, input, *},
};
use space_game_protocol::{PlayerInput, ProtocolPlugin};

use crate::network::{GuestConnection, policy, recovery};

fn headless_client() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        bevy::input::InputPlugin,
        TransformPlugin,
        ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        },
        ProtocolPlugin,
        crate::input::ClientInputPlugin,
    ));
    app.insert_resource(policy::timeline_config());
    app.insert_resource(policy::prediction_manager());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app.finish();
    app.cleanup();
    app.update();
    app
}

#[derive(Resource)]
struct OldSession {
    client: Entity,
    player: Entity,
    unmatched_shot: Entity,
}

fn assert_clean_before_receive(
    old: Res<OldSession>,
    entities: Query<
        Entity,
        bevy::ecs::query::Allow<lightyear::prediction::despawn::PredictionDisable>,
    >,
    connection: Res<GuestConnection>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    sync: Res<LocalTimelineSync>,
) {
    assert!(!entities.contains(old.client));
    assert!(!entities.contains(old.player));
    assert!(!entities.contains(old.unmatched_shot));
    assert!(connection.client.is_none());
    assert!(connection.pending.is_none());
    assert!(!keyboard.pressed(KeyCode::Space));
    assert!(!mouse.pressed(MouseButton::Left));
    assert!(!mouse.pressed(MouseButton::Right));
    assert!(!sync.is_synced());
}

#[test]
fn retirement_cleans_replication_inputs_and_pending_credentials_before_preupdate() {
    let mut app = headless_client();
    let client = app
        .world_mut()
        .spawn((
            Client,
            ReplicationReceiver,
            RemoteId(PeerId::Netcode(42)),
            Connected,
        ))
        .id();
    let player = app
        .world_mut()
        .spawn((
            Replicated,
            Predicted,
            input::native::InputMarker::<PlayerInput>::default(),
            input::native::ActionState(PlayerInput::default()),
            GlobalTransform::default(),
        ))
        .id();
    let (sender, receiver) = crossbeam_channel::bounded(1);
    app.insert_resource(GuestConnection {
        client: Some(client),
        pending: Some(receiver),
        started_by_play: true,
        ..default()
    });
    let unmatched_shot = app
        .world_mut()
        .spawn((
            PreSpawned::new(123),
            lightyear::prediction::despawn::PredictionDisable,
        ))
        .id();
    app.insert_resource(OldSession {
        client,
        player,
        unmatched_shot,
    });
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    app.world_mut()
        .resource_mut::<LocalTimelineSync>()
        .set_synced(true);
    app.world_mut()
        .resource_mut::<StateRollbackMetadata>()
        .request_forced_rollback(Tick(1));
    app.world_mut().spawn((
        Window {
            focused: false,
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        300,
    )));
    recovery::install(&mut app);
    // The assertions run ahead of network receive, not in Update after rollback.
    app.add_systems(
        PreUpdate,
        assert_clean_before_receive.before(lightyear::link::LinkSystems::Receive),
    );
    app.update();
    assert!(
        app.world()
            .resource::<recovery::Suspension>()
            .is_suspended()
    );
    assert!(sender.send(Err("obsolete credentials".to_owned())).is_err());
    assert_eq!(
        app.world()
            .resource::<PredictionManager>()
            .rollback_policy
            .max_rollback_ticks,
        policy::PREDICTION_TICKS
    );
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn a_long_frame_before_play_does_not_start_a_guest_connection() {
    let mut app = headless_client();
    app.insert_resource(GuestConnection::default());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        300,
    )));
    recovery::install(&mut app);
    app.update();
    let connection = app.world().resource::<GuestConnection>();
    assert!(connection.client.is_none());
    assert!(connection.pending.is_none());
    assert_eq!(connection.message, "");
    assert!(!connection.can_retry);
}

#[test]
fn input_redundancy_is_explicit_in_the_installed_protocol() {
    let app = headless_client();
    assert_eq!(
        app.world()
            .resource::<input::InputConfig<PlayerInput>>()
            .packet_redundancy,
        15
    );
}
