use avian2d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;

use project_protocol::{ArenaBoundary, Player, PlayerInput};

use crate::{ARENA_RESIZE_SPEED, PLAYER_RADIUS, PlayerBundle, SERVER_UPS, arena_radius};

use crate::tests::support::arena_simulation;

#[test]
fn players_cannot_walk_through_any_part_of_the_circle() {
    let mut app = arena_simulation();
    *app.world_mut()
        .query::<&mut ArenaBoundary>()
        .single_mut(app.world_mut())
        .expect("arena") = ArenaBoundary::new(arena_radius(32));
    let players: Vec<_> = (0..32_u16)
        .map(|index| {
            let direction = Vec2::from_angle(f32::from(index) * std::f32::consts::TAU / 32.0);
            let mut input = ActionState::<PlayerInput>::default();
            input.0.movement = direction;
            app.world_mut()
                .spawn((
                    PlayerBundle::new(direction * (arena_radius(32) - PLAYER_RADIUS - 1.0)),
                    input,
                ))
                .id()
        })
        .collect();
    for _ in 0..300 {
        app.update();
        for &player in &players {
            let position = app.world().get::<Position>(player).expect("player pose");
            assert!(position.length() + PLAYER_RADIUS <= arena_radius(32) + 0.001);
        }
    }
    for player in players {
        let position = app.world().get::<Position>(player).expect("player pose");
        assert!((position.length() + PLAYER_RADIUS - arena_radius(32)).abs() < 0.01);
        let velocity = app
            .world()
            .get::<LinearVelocity>(player)
            .expect("player velocity");
        assert!(velocity.dot(position.normalize()) < 0.001);
    }
}

#[test]
fn population_changes_resize_and_contain_existing_players() {
    let mut app = arena_simulation();
    *app.world_mut()
        .query::<&mut ArenaBoundary>()
        .single_mut(app.world_mut())
        .expect("arena") = ArenaBoundary::new(arena_radius(4));
    let survivor = app
        .world_mut()
        .spawn(PlayerBundle::new(Vec2::new(
            arena_radius(4) - PLAYER_RADIUS,
            0.0,
        )))
        .id();
    let others: Vec<_> = (0..3_u16)
        .map(|index| {
            app.world_mut()
                .spawn(PlayerBundle::new(Vec2::new(f32::from(index) * 80.0, 0.0)))
                .id()
        })
        .collect();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&ArenaBoundary>()
            .single(app.world())
            .expect("arena")
            .radius,
        arena_radius(4)
    );
    assert!(app.world().get::<Position>(survivor).expect("pose").x > 1300.0);
    for player in others {
        app.world_mut().despawn(player);
    }
    app.update();
    let shrinking = *app
        .world_mut()
        .query::<&ArenaBoundary>()
        .single(app.world())
        .expect("arena");
    assert_eq!(shrinking.target_radius, arena_radius(1));
    assert!(shrinking.radius < arena_radius(4));
    assert!(shrinking.radius > arena_radius(1));
    let mut previous = shrinking.radius;
    for _ in 0..600 {
        app.update();
        let radius = app
            .world_mut()
            .query::<&ArenaBoundary>()
            .single(app.world())
            .expect("arena")
            .radius;
        assert!(radius <= previous);
        assert!(previous - radius <= ARENA_RESIZE_SPEED / SERVER_UPS as f32 + 0.001);
        assert!(
            app.world()
                .get::<Position>(survivor)
                .expect("pose")
                .length()
                + PLAYER_RADIUS
                <= radius + 0.001
        );
        previous = radius;
    }
    assert_eq!(previous, arena_radius(1));
    assert!(
        app.world()
            .get::<Position>(survivor)
            .expect("pose")
            .length()
            + PLAYER_RADIUS
            <= arena_radius(1) + 0.001
    );
    app.world_mut().despawn(survivor);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Player>>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&ArenaBoundary>()
            .single(app.world())
            .expect("arena")
            .radius,
        arena_radius(0)
    );
}
