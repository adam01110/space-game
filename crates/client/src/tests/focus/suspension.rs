use std::time::Duration;

use avian2d::prelude::{LinearVelocity, Position};
use bevy::{prelude::*, time::TimeUpdateStrategy};
use lightyear::prelude::{
    ConfirmedHistory, FrameInterpolationHistory, Predicted, Tick, input::native::ActionState,
};

use project_game::{PlayerBundle, SERVER_UPS};
use project_protocol::{ArenaBoundary, BlasterShot, PlayerInput};

use crate::focus::{FocusPrediction, recent_checkpoint};

use super::{checkpoint, client, focus, seed};

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
