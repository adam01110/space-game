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

pub fn install_arena_interpolation(app: &mut App) {
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

pub fn advance_arena(arena: &mut ArenaBoundary, delta_seconds: f32) {
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

pub fn contain_predicted_bodies(
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
pub fn contain_circle(radius: f32, body_radius: f32, position: &mut Vec2, velocity: &mut Vec2) {
    let limit = (radius - body_radius).max(0.0);
    let distance = position.length();

    if distance > limit {
        let normal = *position / distance;

        *position = normal * limit;
        *velocity -= normal * velocity.dot(normal).max(0.0);
    }
}
