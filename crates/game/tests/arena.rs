use avian2d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use lightyear::prelude::{FrameInterpolate, Predicted};

use project_game::{
    ARENA_GROWTH_MULTIPLIER, ARENA_RESIZE_SPEED, BASE_ARENA_RADIUS, advance_arena, arena_radius,
    contain_circle, contain_predicted_bodies, install_arena_interpolation,
};
use project_protocol::{ArenaBoundary, CircleBody};

#[test]
fn resizing_is_rate_limited_reversible_and_stops_at_target() {
    let mut arena = ArenaBoundary::new(BASE_ARENA_RADIUS);
    arena.target_radius = arena_radius(4);
    advance_arena(&mut arena, 0.25);
    assert_eq!(arena.radius, BASE_ARENA_RADIUS + ARENA_RESIZE_SPEED * 0.25);
    arena.target_radius = BASE_ARENA_RADIUS;
    advance_arena(&mut arena, 0.125);
    assert_eq!(arena.radius, BASE_ARENA_RADIUS + ARENA_RESIZE_SPEED * 0.125);
    advance_arena(&mut arena, 1.0);
    assert_eq!(arena.radius, BASE_ARENA_RADIUS);
    arena.target_radius = BASE_ARENA_RADIUS + 1.0;
    advance_arena(&mut arena, 1.0);
    assert_eq!(arena.radius, arena.target_radius);
}

#[test]
fn render_interpolation_does_not_change_next_simulation_radius() {
    use lightyear::{
        frame_interpolation::FrameInterpolationPlugin, prelude::FrameInterpolationHistory,
    };
    let mut app = App::new();
    app.insert_resource(Time::<Fixed>::from_seconds(1.0));
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .accumulate_overstep(std::time::Duration::from_millis(250));
    app.add_plugins(FrameInterpolationPlugin);
    install_arena_interpolation(&mut app);
    let entity = app
        .world_mut()
        .spawn((
            ArenaBoundary::new(1004.0),
            FrameInterpolate,
            FrameInterpolationHistory::<ArenaBoundary> {
                previous_value: Some(ArenaBoundary::new(1000.0)),
                current_value: Some(ArenaBoundary::new(1004.0)),
            },
        ))
        .id();
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(
        app.world()
            .get::<ArenaBoundary>(entity)
            .expect("render radius")
            .radius,
        1001.0
    );
    app.world_mut().run_schedule(RunFixedMainLoop);
    assert_eq!(
        app.world()
            .get::<ArenaBoundary>(entity)
            .expect("simulation radius")
            .radius,
        1004.0
    );
}

#[test]
fn radius_scales_population_growth_without_changing_solo_size() {
    assert_eq!(arena_radius(0), BASE_ARENA_RADIUS);
    assert_eq!(arena_radius(1), BASE_ARENA_RADIUS);
    assert_eq!(
        arena_radius(4),
        BASE_ARENA_RADIUS * (1.0 + ARENA_GROWTH_MULTIPLIER)
    );
    assert_eq!(
        arena_radius(9),
        BASE_ARENA_RADIUS * (1.0 + 2.0 * ARENA_GROWTH_MULTIPLIER)
    );
}

#[test]
fn contains_full_body_at_every_angle_even_after_large_steps() {
    for angle in 0..360_u16 {
        let normal = Vec2::from_angle(f32::from(angle).to_radians());
        let mut position = normal * 100_000.0;
        let mut velocity = normal * 20_000.0;
        contain_circle(768.0, 23.4, &mut position, &mut velocity);
        assert!(position.length() + 23.4 <= 768.001);
        assert!(velocity.length() < 0.01);
    }
}

#[test]
fn shrink_preserves_tangential_and_inward_motion() {
    let mut position = Vec2::new(2000.0, 0.0);
    let mut velocity = Vec2::new(-20.0, 50.0);
    contain_circle(768.0, 23.4, &mut position, &mut velocity);
    assert_eq!(position, Vec2::new(744.6, 0.0));
    assert_eq!(velocity, Vec2::new(-20.0, 50.0));
    velocity = Vec2::new(20.0, 50.0);
    position.x = 800.0;
    contain_circle(768.0, 23.4, &mut position, &mut velocity);
    assert_eq!(velocity, Vec2::new(0.0, 50.0));
}

#[test]
fn prediction_uses_predicted_radius_and_ignores_confirmed_bodies() {
    let mut app = App::new();
    app.add_systems(Update, contain_predicted_bodies);
    app.world_mut().spawn(ArenaBoundary::new(2000.0));
    let arena = app
        .world_mut()
        .spawn((ArenaBoundary::new(768.0), Predicted))
        .id();
    let predicted = app
        .world_mut()
        .spawn((
            CircleBody::dynamic(23.4),
            Position(Vec2::X * 3000.0),
            LinearVelocity(Vec2::X * 10_000.0),
            Predicted,
        ))
        .id();
    let confirmed = app
        .world_mut()
        .spawn((
            CircleBody::dynamic(23.4),
            Position(Vec2::X * 3000.0),
            LinearVelocity(Vec2::X * 10_000.0),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<Position>(predicted)
            .expect("predicted pose")
            .length()
            < 745.0
    );
    assert_eq!(
        app.world()
            .get::<Position>(confirmed)
            .expect("confirmed pose")
            .x,
        3000.0
    );
    // A corrected/rolled-back radius must immediately drive the same constraint.
    app.world_mut()
        .get_mut::<ArenaBoundary>(arena)
        .expect("arena")
        .radius = 400.0;
    app.update();
    assert!(
        app.world()
            .get::<Position>(predicted)
            .expect("predicted pose")
            .length()
            < 377.0
    );
}

#[test]
fn interior_motion_is_unchanged() {
    let mut position = Vec2::ZERO;
    let mut velocity = Vec2::new(100.0, 200.0);
    contain_circle(768.0, 23.4, &mut position, &mut velocity);
    assert_eq!(position, Vec2::ZERO);
    assert_eq!(velocity, Vec2::new(100.0, 200.0));
}
