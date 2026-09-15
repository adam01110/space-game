use avian2d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::{AppInterpolationExt, FrameInterpolate, InterpolationFns, Predicted};

use project_protocol::{ArenaBoundary, BodyMotion, CircleBody, Player};

// Radius for an empty or solo arena, in world units.
pub const BASE_ARENA_RADIUS: f32 = 1200.0;
// 1.0 square-root scaling; 2.0 doubles the added radius.
pub const ARENA_GROWTH_MULTIPLIER: f32 = 1.0;
// Maximum radius change in world units per second, for both growth and shrinkage.
pub const ARENA_RESIZE_SPEED: f32 = 240.0;

type BoundaryBody<'a> = (&'a CircleBody, &'a mut Position, &'a mut LinearVelocity);

pub(super) fn install_arena_interpolation(app: &mut App) {
    app.interpolate_with::<ArenaBoundary>(InterpolationFns::no_history(|start, end, t| {
        ArenaBoundary {
            radius: start.radius + (end.radius - start.radius) * t,
            target_radius: end.target_radius,
        }
    }));
}

pub(super) fn prepare_predicted_arena(
    trigger: On<Insert, (ArenaBoundary, Predicted)>,
    arenas: Query<(), (With<ArenaBoundary>, With<Predicted>)>,
    mut commands: Commands,
) {
    if arenas.contains(trigger.entity) {
        commands.entity(trigger.entity).insert(FrameInterpolate);
    }
}

pub(super) fn advance_predicted_arena(
    time: Res<Time<Fixed>>,
    mut arenas: Query<&mut ArenaBoundary, With<Predicted>>,
) {
    for mut arena in &mut arenas {
        advance_arena(&mut arena, time.delta_secs());
    }
}

fn advance_arena(arena: &mut ArenaBoundary, delta_seconds: f32) {
    let step = ARENA_RESIZE_SPEED * delta_seconds;
    arena.radius += (arena.target_radius - arena.radius).clamp(-step, step);
}

#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "Player counts are small and world coordinates use f32"
)]
pub fn arena_radius(player_count: usize) -> f32 {
    let population_growth = (player_count.max(1) as f32).sqrt() - 1.0;
    BASE_ARENA_RADIUS * (1.0 + ARENA_GROWTH_MULTIPLIER * population_growth)
}

pub(super) fn resize_arena(
    time: Res<Time<Fixed>>,
    players: Query<(), With<Player>>,
    mut arenas: Query<&mut ArenaBoundary>,
) {
    let target_radius = arena_radius(players.iter().len());
    for mut arena in &mut arenas {
        if arena.target_radius != target_radius {
            info!(
                players = players.iter().len(),
                target_radius, "Arena resize target changed"
            );
        }

        let mut next = *arena;
        next.target_radius = target_radius;

        advance_arena(&mut next, time.delta_secs());
        arena.set_if_neq(next);
    }
}

pub(super) fn contain_authoritative_bodies(
    arenas: Query<&ArenaBoundary>,
    mut bodies: Query<BoundaryBody>,
) {
    if let Ok(arena) = arenas.single() {
        contain_bodies(arena, &mut bodies);
    }
}

pub(super) fn contain_predicted_bodies(
    arenas: Query<&ArenaBoundary, With<Predicted>>,
    mut bodies: Query<BoundaryBody, With<Predicted>>,
) {
    if let Ok(arena) = arenas.single() {
        contain_bodies(arena, &mut bodies);
    }
}

fn contain_bodies<F: QueryFilter>(arena: &ArenaBoundary, bodies: &mut Query<BoundaryBody, F>) {
    for (body, mut position, mut velocity) in bodies {
        if body.motion == BodyMotion::Dynamic {
            contain_circle(arena.radius, body.radius, &mut position.0, &mut velocity.0);
        }
    }
}

/*
An analytic interior constraint, not a polygon or a solid disk collider. Apply after
contact solving and before pose writeback/history on both timelines. No wall thickness,
gaps or tunnelling: even a boost or a shrinking arena cannot leave a body outside.
Remove only outward velocity so inward movement and tangential sliding remain free.
*/
fn contain_circle(radius: f32, body_radius: f32, position: &mut Vec2, velocity: &mut Vec2) {
    let limit = (radius - body_radius).max(0.0);
    let distance = position.length();

    if distance > limit {
        let normal = *position / distance;

        *position = normal * limit;
        *velocity -= normal * velocity.dot(normal).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
