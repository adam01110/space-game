use super::*;
use std::time::Duration;

use avian2d::prelude::{AngularVelocity, LinearVelocity};
use bevy::{ecs::system::RunSystemOnce, state::app::StatesPlugin, time::TimeUpdateStrategy};
use lightyear::prelude::{client::ClientPlugins, input::native::ActionState};
use project_game::{ClientSimulationPlugin, GamePlugin, PlayerBundle, SERVER_UPS};
use project_protocol::{
    ArenaBoundary, BlasterReload, BlasterShot, BlasterTrigger, PlayerBlasters, PlayerBoost,
    PlayerHealth, PlayerInput, PlayerPhaseBeam, ProtocolPlugin,
};

fn client() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        TransformPlugin,
        ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
        },
        ProtocolPlugin,
        GamePlugin,
        ClientSimulationPlugin,
        ClientFocusPlugin,
    ));
    app.insert_resource(PredictionManager::default());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));
    app.world_mut().spawn((
        Window {
            focused: false,
            ..default()
        },
        PrimaryWindow,
    ));
    app.finish();
    app.cleanup();
    app.update();
    app
}

fn focus(app: &mut App, focused: bool) {
    let world = app.world_mut();
    let mut query = world.query::<&mut Window>();
    query.single_mut(world).expect("window").focused = focused;
    world.run_system_once(observe_focus).expect("observe focus");
}

/// Complete a checkpoint the way replication does: record it, then confirm it.
fn checkpoint(app: &mut App, tick: u32) {
    let mut map = app.world_mut().resource_mut::<ReplicationCheckpointMap>();
    let replication_tick = map.last_confirmed_replicon_tick().unwrap_or_default() + 1;
    map.record(replication_tick, Tick(tick));
    assert_eq!(
        map.record_last_confirmed_tick(replication_tick),
        Some(Tick(tick))
    );
}

fn seed<C: Component + Clone + PartialEq>(app: &mut App, entity: Entity, tick: Tick, value: C) {
    let mut history = ConfirmedHistory::<C>::default();
    history.insert_present(tick, value);
    app.world_mut().entity_mut(entity).insert(history);
}

#[test]
fn checkpoint_must_be_new_complete_and_within_replay_budget() {
    assert_eq!(recent_checkpoint(None, None, Tick(100), 20), None);
    assert_eq!(
        recent_checkpoint(Some(Tick(95)), Some(Tick(95)), Tick(100), 20),
        None
    );
    assert_eq!(recent_checkpoint(None, Some(Tick(79)), Tick(100), 20), None);
    assert_eq!(
        recent_checkpoint(None, Some(Tick(101)), Tick(100), 20),
        None
    );
    assert_eq!(
        recent_checkpoint(Some(Tick(10)), Some(Tick(98)), Tick(100), 20),
        Some(Tick(98))
    );
    assert_eq!(
        recent_checkpoint(None, Some(Tick(u32::MAX)), Tick(2), 20),
        None
    );
}

#[test]
fn one_fps_background_frames_do_not_simulate_gameplay_or_physics() {
    let mut app = client();
    let player = app
        .world_mut()
        .spawn((
            PlayerBundle::new(Vec2::new(100.0, 0.0)),
            Predicted,
            ActionState::<PlayerInput>::default(),
        ))
        .id();
    app.world_mut()
        .entity_mut(player)
        .insert(LinearVelocity(Vec2::X * 500.0));
    let arena = app
        .world_mut()
        .spawn((
            Predicted,
            ArenaBoundary {
                radius: 1200.0,
                target_radius: 2000.0,
            },
        ))
        .id();
    let shot = app
        .world_mut()
        .spawn(BlasterShot {
            position: Vec2::ZERO,
            direction: Vec2::X,
            ticks_left: 60,
        })
        .id();
    let fixed_before = app.world().resource::<Time<Fixed>>().elapsed();
    for _ in 0..60 {
        app.update();
    }
    assert!(
        app.world().resource::<Time<Fixed>>().elapsed() > fixed_before + Duration::from_secs(10)
    );
    assert_eq!(
        app.world().get::<Position>(player).expect("position").0,
        Vec2::new(100.0, 0.0)
    );
    assert_eq!(
        app.world()
            .get::<ArenaBoundary>(arena)
            .expect("arena")
            .radius,
        1200.0
    );
    assert_eq!(
        app.world()
            .get::<BlasterShot>(shot)
            .expect("shot")
            .ticks_left,
        60
    );
    assert!(!app.world().resource::<FocusPrediction>().accepts_input());
}

fn check_passive_updates(frame_duration: Duration) {
    let mut app = client();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_duration));
    let player = app
        .world_mut()
        .spawn((
            PlayerBundle::new(Vec2::ZERO),
            Predicted,
            ActionState::<PlayerInput>::default(),
        ))
        .id();
    // Rendering must not restore these old prediction samples over server snapshots.
    app.world_mut()
        .entity_mut(player)
        .insert(FrameInterpolationHistory::<Position> {
            previous_value: Some(Position(Vec2::splat(-999.0))),
            current_value: Some(Position(Vec2::splat(-999.0))),
        });
    for (tick, x) in [(10, 100.0), (20, 200.0), (30, 300.0)] {
        seed(&mut app, player, Tick(tick), Position(Vec2::new(x, 0.0)));
        // A not-yet-completed update must not leak into presentation.
        app.world_mut()
            .get_mut::<ConfirmedHistory<Position>>(player)
            .expect("history")
            .insert_present(Tick(tick + 1), Position(Vec2::splat(999.0)));
        checkpoint(&mut app, tick);
        app.update();
        assert!(!app.world().resource::<FocusPrediction>().accepts_input());
        assert_eq!(
            app.world().get::<Position>(player).expect("position").0,
            Vec2::new(x, 0.0)
        );
        assert_eq!(
            app.world()
                .get::<Transform>(player)
                .expect("render transform")
                .translation
                .truncate(),
            Vec2::new(x, 0.0)
        );
    }
}

#[test]
fn visible_unfocused_clients_display_server_movement() {
    check_passive_updates(Duration::from_secs_f64(1.0 / SERVER_UPS));
}

#[test]
fn compositor_throttled_clients_display_server_movement() {
    check_passive_updates(Duration::from_secs(1));
}

#[test]
fn active_prediction_is_not_overwritten_by_passive_presentation() {
    let mut app = client();
    focus(&mut app, true);
    app.insert_resource(FocusPrediction::default());
    let player = app
        .world_mut()
        .spawn((
            PlayerBundle::new(Vec2::ZERO),
            Predicted,
            ActionState::<PlayerInput>::default(),
        ))
        .id();
    seed(&mut app, player, Tick(10), Position(Vec2::splat(500.0)));
    checkpoint(&mut app, 10);
    app.update();
    assert!(app.world().resource::<FocusPrediction>().accepts_input());
    assert_eq!(
        app.world()
            .get::<Position>(player)
            .expect("predicted position")
            .0,
        Vec2::ZERO
    );
}

#[test]
fn focus_waits_for_a_new_checkpoint_and_missing_history() {
    let mut app = client();
    checkpoint(&mut app, 90);
    app.world_mut()
        .resource_mut::<LocalTimelineSync>()
        .set_synced(true);
    app.world_mut()
        .resource_mut::<LocalTimeline>()
        .apply_delta(100);
    focus(&mut app, true);
    request_restoration(app.world_mut());
    assert!(matches!(
        app.world().resource::<FocusPrediction>().phase,
        PredictionPhase::Waiting { .. }
    ));
    let entity = app.world_mut().spawn((Predicted, Position(Vec2::X))).id();
    checkpoint(&mut app, 98);
    request_restoration(app.world_mut());
    assert!(matches!(
        app.world().resource::<FocusPrediction>().phase,
        PredictionPhase::Waiting { .. }
    ));
    let mut history = ConfirmedHistory::<Position>::default();
    history.insert_present(Tick(98), Position(Vec2::Y));
    app.world_mut().entity_mut(entity).insert(history);
    request_restoration(app.world_mut());
    assert!(matches!(
        app.world().resource::<FocusPrediction>().phase,
        PredictionPhase::Restoring { tick: Tick(98), .. }
    ));
    assert_eq!(
        app.world()
            .resource::<StateRollbackMetadata>()
            .forced_rollback_tick(),
        Some(Tick(98))
    );
    // Consuming a request alone is not proof that restoration ran.
    app.world_mut()
        .run_system_once(finish_restoration)
        .expect("finish");
    assert!(!app.world().resource::<FocusPrediction>().accepts_input());
}

#[test]
fn long_pause_restores_authority_and_replays_only_the_recent_lead() {
    let mut app = client();
    let player = app
        .world_mut()
        .spawn((
            PlayerBundle::new(Vec2::new(100.0, 0.0)),
            Predicted,
            ActionState::<PlayerInput>::default(),
        ))
        .id();
    for _ in 0..60 {
        app.update();
    }
    let before = app.world().resource::<LocalTimeline>().tick();
    let resumed_tick = Tick(1000);
    app.world_mut()
        .resource_mut::<LocalTimeline>()
        .apply_delta(resumed_tick - before);
    app.world_mut()
        .resource_mut::<LocalTimelineSync>()
        .set_synced(true);
    let link = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<NetworkingMetadata>().mode = NetworkTopology::Client(link);
    focus(&mut app, true);
    let target = Tick(998);
    // Replication can add and remove entities while prediction is suspended.
    let gone = app.world_mut().spawn((Predicted, BlasterTrigger(7))).id();
    app.world_mut().despawn(gone);
    let arena = app
        .world_mut()
        .spawn((
            Predicted,
            ArenaBoundary {
                radius: 900.0,
                target_radius: 900.0,
            },
        ))
        .id();
    seed(
        &mut app,
        arena,
        target,
        ArenaBoundary {
            radius: 1200.0,
            target_radius: 1500.0,
        },
    );
    seed(&mut app, player, target, Position(Vec2::new(500.0, 200.0)));
    seed(&mut app, player, target, Rotation::default());
    seed(&mut app, player, target, LinearVelocity::ZERO);
    seed(&mut app, player, target, AngularVelocity::ZERO);
    seed(&mut app, player, target, PlayerBlasters::default());
    seed(&mut app, player, target, BlasterTrigger(42));
    seed(&mut app, player, target, BlasterReload::default());
    seed(&mut app, player, target, PlayerBoost::default());
    seed(&mut app, player, target, PlayerHealth::default());
    seed(&mut app, player, target, PlayerPhaseBeam::default());
    checkpoint(&mut app, 998);
    // Restore must not trust these stale live values.
    app.world_mut().entity_mut(player).insert((
        Position(Vec2::splat(-999.0)),
        LinearVelocity(Vec2::splat(900.0)),
        BlasterTrigger(0),
    ));
    let metrics_before = app.world().resource::<PredictionMetrics>().rollback_ticks;
    app.world_mut().run_schedule(PreUpdate);
    assert!(app.world().resource::<FocusPrediction>().accepts_input());
    assert_eq!(
        app.world().get::<Position>(player).expect("position").0,
        Vec2::new(500.0, 200.0)
    );
    assert_eq!(
        app.world()
            .get::<LinearVelocity>(player)
            .expect("velocity")
            .0,
        Vec2::ZERO
    );
    assert_eq!(
        app.world()
            .get::<BlasterTrigger>(player)
            .expect("trigger")
            .0,
        42
    );
    // Only the lead from the checkpoint to the local tick is replayed.
    assert_eq!(
        app.world().resource::<PredictionMetrics>().rollback_ticks - metrics_before,
        (resumed_tick - target).cast_unsigned()
    );
    assert!(app.world().get_entity(gone).is_err());
    let restored_radius = app
        .world()
        .get::<ArenaBoundary>(arena)
        .expect("arena")
        .radius;
    assert!(restored_radius > 1200.0 && restored_radius < 1210.0);
    // The next full rendered frame must not overwrite restored poses with stale
    // frame-interpolation data or return to a suspended simulation.
    app.update();
    assert!(app.world().resource::<FocusPrediction>().accepts_input());
    assert_eq!(
        app.world().get::<Position>(player).expect("position").0,
        Vec2::new(500.0, 200.0)
    );
}

#[test]
fn focus_loss_during_recovery_stays_suspended() {
    let mut app = client();
    app.world_mut().resource_mut::<FocusPrediction>().phase = PredictionPhase::Restoring {
        tick: Tick(10),
        prepared: true,
    };
    focus(&mut app, false);
    app.world_mut()
        .run_system_once(finish_restoration)
        .expect("finish");
    assert!(matches!(
        app.world().resource::<FocusPrediction>().phase,
        PredictionPhase::Suspended
    ));
}
