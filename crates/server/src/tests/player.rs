use avian2d::prelude::Position;
use bevy::prelude::*;
use lightyear::{
    connection::client::Connected,
    prelude::{server::*, *},
};
use rand::{SeedableRng, rngs::StdRng};

use project_game::PLAYER_RADIUS;
use project_protocol::ArenaBoundary;

use crate::{SPAWN_CLEARANCE, find_spawn_position, spawn_player_for_client};

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
