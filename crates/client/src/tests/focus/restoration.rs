use avian2d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use lightyear::prelude::{
    ConfirmedHistory, LocalTimeline, LocalTimelineSync, NetworkTopology, NetworkingMetadata,
    Predicted, PredictionMetrics, StateRollbackMetadata, Tick, input::native::ActionState,
};

use project_game::PlayerBundle;
use project_protocol::{
    ArenaBoundary, BlasterReload, BlasterTrigger, PlayerBlasters, PlayerBoost, PlayerHealth,
    PlayerInput, PlayerPhaseBeam,
};

use crate::focus::{FocusPrediction, PredictionPhase, finish_restoration, request_restoration};

use super::{checkpoint, client, focus, seed};

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
