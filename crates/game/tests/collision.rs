use std::time::Duration;

use avian2d::prelude::*;
use bevy::{
    prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy, transform::TransformPlugin,
};
use lightyear::prelude::{input::native::ActionState, server::ServerPlugins, *};
use project_game::{GamePlugin, PLAYER_RADIUS, PlayerBundle, SERVER_UPS, ServerSimulationPlugin};
use project_protocol::{CircleBody, PlayerInput, ProtocolPlugin};

fn app() -> App {
    let mut app = App::new();
    let tick = Duration::from_secs_f64(1.0 / SERVER_UPS);
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        TransformPlugin,
        ServerPlugins {
            tick_duration: tick,
        },
        ProtocolPlugin,
        GamePlugin,
        ServerSimulationPlugin,
    ));
    // ServerPlugins has no visual interpolation; enable it here to exercise the shared
    // render/physics boundary without opening a window or requiring a network connection.
    app.add_plugins(lightyear::frame_interpolation::FrameInterpolationPlugin);
    app.init_resource::<PeerMetadata>();
    app.insert_resource(ReplicationMetadata::new(tick));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(tick));
    app.finish();
    app.cleanup();
    app.update();
    app
}

fn player(app: &mut App, position: Vec2, movement: Vec2) -> Entity {
    app.world_mut()
        .spawn((
            PlayerBundle::new(position),
            ActionState(PlayerInput {
                movement,
                aim: Vec2::Y,
            }),
        ))
        .id()
}

fn position(app: &App, entity: Entity) -> Vec2 {
    app.world().get::<Position>(entity).unwrap().0
}

#[test]
fn head_on_players_stop_without_crossing_and_release_immediately() {
    let mut app = app();
    let left = player(&mut app, Vec2::new(-80.0, 0.0), Vec2::X);
    let right = player(&mut app, Vec2::new(80.0, 0.0), Vec2::NEG_X);
    for _ in 0..120 {
        app.update();
        let (a, b) = (position(&app, left), position(&app, right));
        assert!(a.x < b.x, "players crossed: {a:?}, {b:?}");
        // Avian is a numerical solver; allow subpixel contact slop.
        assert!(
            a.distance(b) >= PLAYER_RADIUS * 2.0 - 0.5,
            "players penetrated: {a:?}, {b:?}"
        );
    }
    let stopped = position(&app, left);
    app.world_mut()
        .get_mut::<ActionState<PlayerInput>>(left)
        .unwrap()
        .0
        .movement = Vec2::NEG_X;
    app.update();
    assert!(
        position(&app, left).x < stopped.x - 1.0,
        "player stuck after leaving contact"
    );
}

#[test]
fn generic_static_circle_blocks_player_and_does_not_move() {
    let mut app = app();
    let obstacle = app
        .world_mut()
        .spawn((
            CircleBody::fixed(30.0),
            Position::default(),
            Rotation::default(),
        ))
        .id();
    let moving = player(&mut app, Vec2::new(-100.0, 0.0), Vec2::X);
    for _ in 0..120 {
        app.update();
        assert!(position(&app, moving).x <= -(30.0 + PLAYER_RADIUS) + 0.5);
        assert_eq!(position(&app, obstacle), Vec2::ZERO);
    }
    assert_eq!(
        app.world().get::<RigidBody>(obstacle),
        Some(&RigidBody::Static)
    );
}

#[test]
fn free_motion_is_integrated_once_and_visual_transform_cannot_move_physics() {
    let mut app = app();
    let moving = player(&mut app, Vec2::ZERO, Vec2::X);
    app.update();
    let before = position(&app, moving);
    app.world_mut()
        .get_mut::<Transform>(moving)
        .unwrap()
        .translation = Vec3::splat(10000.0);
    app.update();
    let after = position(&app, moving);
    assert!((after.x - before.x - 512.0 / SERVER_UPS as f32).abs() < 0.001);
    assert_eq!(after.y, 0.0);
    assert_eq!(
        app.world()
            .get::<Transform>(moving)
            .unwrap()
            .translation
            .truncate(),
        after
    );
}

#[test]
fn physics_and_replication_share_canonical_pose_and_velocity() {
    let app = app();
    let registry = app.world().resource::<ComponentRegistry>();
    assert!(registry.is_registered::<Position>());
    assert!(registry.is_registered::<Rotation>());
    assert!(registry.is_registered::<LinearVelocity>());
    assert!(registry.is_registered::<AngularVelocity>());
    assert!(registry.is_registered::<CircleBody>());
}

#[test]
fn rendered_pose_is_linear_and_restored_before_the_next_fixed_loop() {
    let mut app = app();
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs(1)));
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .accumulate_overstep(Duration::from_millis(500));
    let body = player(&mut app, Vec2::new(20.0, 0.0), Vec2::ZERO);
    app.world_mut().entity_mut(body).insert(FrameInterpolate);
    app.world_mut().entity_mut(body).insert((
        FrameInterpolationHistory::<Position> {
            previous_value: Some(Position::from_xy(40.0, 0.0)),
            current_value: Some(Position::from_xy(20.0, 0.0)),
        },
        FrameInterpolationHistory::<Rotation> {
            previous_value: Some(Rotation::default()),
            current_value: Some(Rotation::default()),
        },
        FrameInterpolationHistory::<LinearVelocity> {
            previous_value: Some(LinearVelocity(Vec2::NEG_X * 512.0)),
            current_value: Some(LinearVelocity::default()),
        },
        FrameInterpolationHistory::<AngularVelocity> {
            previous_value: Some(AngularVelocity::default()),
            current_value: Some(AngularVelocity::default()),
        },
    ));
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(
        position(&app, body),
        Vec2::new(30.0, 0.0),
        "contact tangents must not cause Hermite overshoot"
    );
    assert_eq!(
        app.world()
            .get::<Transform>(body)
            .unwrap()
            .translation
            .truncate(),
        Vec2::new(30.0, 0.0)
    );
    app.world_mut().run_schedule(RunFixedMainLoop);
    assert_eq!(
        position(&app, body),
        Vec2::new(20.0, 0.0),
        "render interpolation leaked into physics"
    );
}

#[test]
fn client_body_setup_accepts_either_replication_component_order() {
    use project_game::ClientSimulationPlugin;

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        TransformPlugin,
        lightyear::prelude::client::ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
        },
        ProtocolPlugin,
        GamePlugin,
        ClientSimulationPlugin,
    ));
    app.finish();
    app.cleanup();

    let first = app
        .world_mut()
        .spawn((
            Predicted,
            Position::from_xy(10.0, 20.0),
            Rotation::default(),
        ))
        .id();
    app.world_mut()
        .entity_mut(first)
        .insert(CircleBody::dynamic(PLAYER_RADIUS));
    let second = app
        .world_mut()
        .spawn((
            CircleBody::dynamic(PLAYER_RADIUS),
            Position::from_xy(60.0, 20.0),
            Rotation::default(),
        ))
        .id();
    assert!(app.world().get::<RigidBody>(second).is_none());
    app.world_mut().entity_mut(second).insert(Predicted);
    app.world_mut().flush();

    for entity in [first, second] {
        assert_eq!(
            app.world().get::<RigidBody>(entity),
            Some(&RigidBody::Dynamic)
        );
        assert!(
            app.world()
                .get::<Collider>(entity)
                .unwrap()
                .shape()
                .as_ball()
                .is_some()
        );
        assert!(app.world().get::<FrameInterpolate>(entity).is_some());
    }
    assert_eq!(position(&app, first), Vec2::new(10.0, 20.0));
    assert_eq!(position(&app, second), Vec2::new(60.0, 20.0));
}
