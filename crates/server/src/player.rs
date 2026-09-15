use avian2d::prelude::Position;
use bevy::prelude::*;
use lightyear::{connection::client::Connected, prelude::server::*, prelude::*};
use rand::{Rng, RngExt};

use project_game::{PLAYER_RADIUS, PlayerBundle, arena_radius};
use project_protocol::{ArenaBoundary, CircleBody};

const SPAWN_CLEARANCE: f32 = 2.0;
const SPAWN_ATTEMPTS: usize = 256;

pub(super) struct ServerPlayerPlugin;

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

fn spawn_player_for_client(
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

fn find_spawn_position(
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn random_spawns_cover_the_disk_uniformly_by_area() {
        let arena = ArenaBoundary::new(1200.0);
        let limit = arena.radius - PLAYER_RADIUS - SPAWN_CLEARANCE;
        let mut rng = StdRng::seed_from_u64(42);
        let mut mean = Vec2::ZERO;
        let mut mean_squared_radius = 0.0;
        let mut inner_count = 0;
        for _ in 0..4096 {
            let position = find_spawn_position(arena, &[], &mut rng).expect("empty arena");
            assert!(position.length() <= limit);
            let normalized = position / limit;
            mean += normalized / 4096.0;
            mean_squared_radius += normalized.length_squared() / 4096.0;
            if normalized.length() < 0.5 {
                inner_count += 1;
            }
        }
        assert!(mean.length() < 0.04);
        assert!((mean_squared_radius - 0.5).abs() < 0.03);
        // The inner half-radius occupies one quarter of the disk area.
        assert!((900..1150).contains(&inner_count));
    }

    #[test]
    fn simultaneous_connection_spawns_are_reserved() {
        let arena = ArenaBoundary::new(1200.0);
        let mut occupied = Vec::new();
        let mut rng = StdRng::seed_from_u64(7);

        for _ in 0..64 {
            let position = find_spawn_position(arena, &occupied, &mut rng)
                .expect("base arena has room for simultaneous connections");
            assert!(
                occupied
                    .iter()
                    .all(|(other, radius)| other.distance(position)
                        > PLAYER_RADIUS + radius + SPAWN_CLEARANCE)
            );
            occupied.push((position, PLAYER_RADIUS));
        }

        assert!(occupied.iter().any(|(position, _)| position.x > 600.0));
        assert!(occupied.iter().any(|(position, _)| position.x < -600.0));
    }

    #[test]
    fn batched_connection_observers_spawn_distinct_players() {
        use project_protocol::Player;
        let mut app = App::new();
        app.add_observer(spawn_player_for_client);
        app.world_mut().spawn(ArenaBoundary::new(1200.0));
        {
            let mut commands = app.world_mut().commands();
            for id in 0..8 {
                commands.spawn((RemoteId(PeerId::Netcode(id)), ClientOf, Connected));
            }
        }
        app.world_mut().flush();
        let positions: Vec<_> = app
            .world_mut()
            .query_filtered::<&Position, With<Player>>()
            .iter(app.world())
            .map(|position| position.0)
            .collect();
        assert_eq!(positions.len(), 8);
        assert!(
            positions
                .iter()
                .all(|position| position.length() + PLAYER_RADIUS + SPAWN_CLEARANCE <= 1200.0)
        );
        for (index, position) in positions.iter().enumerate() {
            for other in positions.iter().skip(index + 1) {
                assert!(position.distance(*other) > 2.0 * PLAYER_RADIUS + SPAWN_CLEARANCE);
            }
        }
    }

    #[test]
    fn reconnect_gets_a_fresh_random_position() {
        let arena = ArenaBoundary::new(1200.0);
        let mut rng = StdRng::seed_from_u64(11);
        let first = find_spawn_position(arena, &[], &mut rng).expect("first spawn");
        let reconnected = find_spawn_position(arena, &[], &mut rng).expect("reconnect spawn");
        assert_ne!(reconnected, first);
    }

    #[test]
    fn positions_are_bounded_and_full_arena_returns_none() {
        let arena = ArenaBoundary::new(PLAYER_RADIUS + SPAWN_CLEARANCE);
        let mut rng = StdRng::seed_from_u64(2);
        let center = find_spawn_position(arena, &[], &mut rng).expect("center fits exactly");
        assert!(center.length() + PLAYER_RADIUS + SPAWN_CLEARANCE <= arena.radius);

        let occupied = [(center, PLAYER_RADIUS)];
        assert_eq!(find_spawn_position(arena, &occupied, &mut rng), None);
        for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                find_spawn_position(ArenaBoundary::new(radius), &[], &mut rng),
                None
            );
        }
    }
}
