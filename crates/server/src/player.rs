use avian2d::prelude::Position;
use bevy::prelude::*;
use lightyear::{connection::client::Connected, prelude::server::*, prelude::*};
use rand::{Rng, RngExt};

use project_game::{PLAYER_RADIUS, PlayerBundle, arena_radius};
use project_protocol::{ArenaBoundary, CircleBody};

pub const SPAWN_CLEARANCE: f32 = 2.0;
const SPAWN_ATTEMPTS: usize = 256;

pub struct ServerPlayerPlugin;

impl Plugin for ServerPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_arena)
            .add_observer(spawn_player_for_client);
    }
}

fn spawn_arena(mut commands: Commands) {
    commands.spawn((
        Name::new("Arena boundary"),
        ArenaBoundary::new(arena_radius(0)),
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::All),
    ));
}

pub fn spawn_player_for_client(
    trigger: On<Add, Connected>,
    // ClientOf selects connection entities; RemoteId provides the peer ID assigned as owner.
    clients: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = clients.get(trigger.entity) else {
        return;
    };

    let peer_id = remote_id.0;
    let owner = trigger.entity;
    // Each closure mutates the world immediately, so later joins in the same command batch
    // observe earlier spawns and cannot select the same position.
    commands.queue(move |world: &mut World| {
        let arena = {
            let mut arenas = world.query::<&ArenaBoundary>();
            arenas.single(world).copied()
        };
        let Ok(arena) = arena else {
            warn!("cannot spawn {peer_id:?}: expected exactly one arena boundary");
            return;
        };
        let occupied: Vec<_> = {
            let mut bodies = world.query::<(&Position, &CircleBody)>();
            bodies
                .iter(world)
                .map(|(position, circle)| (position.0, circle.radius))
                .collect()
        };
        let Some(spawn_position) = find_spawn_position(arena, &occupied, &mut rand::rng()) else {
            warn!("cannot spawn {peer_id:?}: no safe position found inside the arena");
            return;
        };

        let player = world
            .spawn((
                Name::new(format!("Player {peer_id:?}")),
                PlayerBundle::new(spawn_position),
                Replicate::to_clients(NetworkTarget::All),
                // Contacts must use the same simulation tick on both sides. Delayed remote
                // interpolation would put the other collider in the past.
                PredictionTarget::to_clients(NetworkTarget::All),
                ControlledBy {
                    owner,
                    lifetime: default(),
                },
            ))
            .id();

        info!("spawned {player:?} for {peer_id:?}");
    });
}

// Use the current physical boundary, not its eventual expansion target.
fn spawnable_radius(arena: ArenaBoundary) -> Option<f32> {
    let limit = arena.radius - PLAYER_RADIUS - SPAWN_CLEARANCE;
    (limit.is_finite() && limit >= 0.0).then_some(limit)
}

fn is_clear_of_occupied(candidate: Vec2, occupied: &[(Vec2, f32)]) -> bool {
    for (position, radius) in occupied {
        if position.distance(candidate) <= PLAYER_RADIUS + radius + SPAWN_CLEARANCE {
            return false;
        }
    }

    true
}

pub fn find_spawn_position(
    arena: ArenaBoundary,
    occupied: &[(Vec2, f32)],
    rng: &mut impl Rng,
) -> Option<Vec2> {
    let limit = spawnable_radius(arena)?;

    // sqrt(U) samples uniformly by disk area instead of clustering near the center.
    // Randomness is server-only; clients receive the resulting authoritative position.
    for _ in 0..SPAWN_ATTEMPTS {
        let angle = rng.random::<f32>() * std::f32::consts::TAU;
        let radius = rng.random::<f32>().sqrt() * limit;

        let candidate = Vec2::from_angle(angle) * radius;
        if candidate.length() <= limit && is_clear_of_occupied(candidate, occupied) {
            return Some(candidate);
        }
    }

    // Never fall back to a biased grid, an overlapping spawn, or an unbounded search.
    None
}
