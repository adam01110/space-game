use avian2d::prelude::{PhysicsSystems, Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::RngExt;

use space_game_game::{BASE_ARENA_RADIUS, BeamDamageCarry};
use space_game_protocol::{ArenaBoundary, Asteroid, AsteroidHealth, CircleBody, Player};

const INNER_COUNT: u16 = 12;
const OUTER_COUNT: u16 = 180;
const MAX_ASTEROIDS: usize = 800;
const RESPAWN_INTERVAL: f32 = 3.0;
const ATTEMPTS: usize = 64;

// Offset from the border, retained when the arena changes size. Only the server moves rocks;
// their authoritative poses are replicated to every client.
#[derive(Component)]
struct Outside {
    distance: f32,
    direction: Vec2,
}

#[derive(Resource, Default)]
struct SpawnClock(f32);

pub(super) struct ServerAsteroidPlugin;

impl Plugin for ServerAsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnClock>()
            .add_observer(populate_on_first_join)
            .add_systems(
                FixedPostUpdate,
                (push_exterior_outward, periodically_replenish_asteroids)
                    .chain()
                    .before(PhysicsSystems::StepSimulation),
            );
    }
}

fn populate_on_first_join(
    _trigger: On<Add, Player>,
    players: Query<(), With<Player>>,
    mut commands: Commands,
) {
    if players.iter().len() == 1 {
        // Run after the player insertion has completed. All peers receive rocks already
        // present in the world on the first replication tick, not three seconds later.
        commands.queue(populate_asteroids);
    }
}

fn push_exterior_outward(
    arena: Query<&ArenaBoundary>,
    mut rocks: Query<(&Outside, &CircleBody, &mut Position)>,
) {
    let Ok(arena) = arena.single() else { return };
    for (outside, body, mut position) in &mut rocks {
        let desired = arena.radius + body.radius + outside.distance;
        // Follow expansion without ever dragging an asteroid back toward a shrinking border.
        if position.0.length_squared() < desired * desired {
            position.0 = outside.direction * desired;
        }
    }
}

fn periodically_replenish_asteroids(world: &mut World) {
    let delta = world.resource::<Time<Fixed>>().delta_secs();
    let mut clock = world.resource_mut::<SpawnClock>();
    clock.0 += delta;
    if clock.0 < RESPAWN_INTERVAL {
        return;
    }
    clock.0 = 0.0;
    populate_asteroids(world);
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Finite arena radii and the population cap bound these counts"
)]
fn density_target(base: u16, scale: f32) -> usize {
    ((f32::from(base) * scale).round() as usize).min(MAX_ASTEROIDS)
}

fn populate_asteroids(world: &mut World) {
    if world
        .query_filtered::<(), With<Player>>()
        .iter(world)
        .next()
        .is_none()
    {
        return;
    }
    let Ok(arena) = world.query::<&ArenaBoundary>().single(world).copied() else {
        return;
    };
    let mut occupied: Vec<_> = world
        .query::<(&Position, &CircleBody)>()
        .iter(world)
        .map(|(position, body)| (position.0, body.radius))
        .collect();
    let mut inside = 0;
    let mut outside = 0;
    let mut total = 0;
    for (position, body, outer) in world
        .query_filtered::<(&Position, &CircleBody, Has<Outside>), With<Asteroid>>()
        .iter(world)
    {
        total += 1;
        let distance = position.0.length();
        if outer {
            // After a shrink, distant rocks stay put. Count only the visible warning band
            // so new ones fill the gap by the pylons without dragging colliders through ships.
            if distance >= arena.radius + body.radius + 24.0
                && distance <= arena.radius + body.radius + 32.0 + 850.0
            {
                outside += 1;
            }
        } else if distance + body.radius + 32.0 <= arena.radius {
            inside += 1;
        }
    }

    // Preserve area density inside and density per unit of circumference in the exterior band.
    let scale = (arena.radius / BASE_ARENA_RADIUS).max(1.0);
    let inner_target = density_target(INNER_COUNT, scale * scale);
    let outer_target = density_target(OUTER_COUNT, scale);

    let mut rng = rand::rng();
    for outer in [false, true] {
        let missing = if outer {
            outer_target.saturating_sub(outside)
        } else {
            inner_target.saturating_sub(inside)
        };
        for _ in 0..missing.min(MAX_ASTEROIDS.saturating_sub(total)) {
            let radius = rng.random_range(23.0..55.0);
            let mut spot = None;
            for _ in 0..ATTEMPTS {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let distance = if outer {
                    arena.radius + radius + 32.0 + rng.random::<f32>() * 850.0
                } else {
                    rng.random::<f32>().sqrt() * (arena.radius - radius - 32.0).max(0.0)
                };
                let candidate = Vec2::from_angle(angle) * distance;
                if occupied
                    .iter()
                    .all(|(p, r)| candidate.distance_squared(*p) > (radius + r + 12.0).powi(2))
                {
                    spot = Some(candidate);
                    break;
                }
            }
            let Some(position) = spot else { continue };
            occupied.push((position, radius));
            total += 1;
            let asteroid = Asteroid {
                variant: rng.random_range(0..3),
                radius,
                stretch: Vec2::new(
                    if rng.random_bool(0.5) { -1.0 } else { 1.0 } * rng.random_range(0.8..1.25),
                    rng.random_range(0.78..1.22),
                ),
                angle: rng.random::<f32>() * std::f32::consts::TAU,
            };
            let mut entity = world.spawn((
                Name::new("Asteroid"),
                asteroid,
                AsteroidHealth(AsteroidHealth::FULL),
                CircleBody::fixed(radius),
                Position(position),
                Rotation::default(),
                BeamDamageCarry::default(),
                Replicate::to_clients(NetworkTarget::All),
                PredictionTarget::to_clients(NetworkTarget::All),
            ));
            if outer {
                entity.insert(Outside {
                    distance: position.length() - arena.radius - radius,
                    direction: position.normalize(),
                });
            }
        }
    }
}
