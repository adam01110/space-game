use bevy::prelude::*;
use space_game_protocol::PlayerHealth;

use crate::PlayerBundle;

use super::support::simulation;

fn spawn_player(app: &mut App, health: u8) -> Entity {
    let player = app.world_mut().spawn(PlayerBundle::new(Vec2::ZERO)).id();
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerHealth(health));
    player
}

fn health(app: &App, player: Entity) -> u8 {
    app.world()
        .get::<PlayerHealth>(player)
        .expect("player health")
        .0
}

#[test]
fn damaged_players_regenerate_one_point_per_second_independently() {
    let mut app = simulation();
    let first = spawn_player(&mut app, 90);

    for _ in 0..30 {
        app.update();
    }
    let second = spawn_player(&mut app, 80);

    for _ in 0..29 {
        app.update();
    }
    assert_eq!(health(&app, first), 90);
    assert_eq!(health(&app, second), 80);

    app.update();
    assert_eq!(health(&app, first), 91);
    assert_eq!(health(&app, second), 80);

    for _ in 0..30 {
        app.update();
    }
    assert_eq!(health(&app, first), 91);
    assert_eq!(health(&app, second), 81);
}

#[test]
fn regeneration_stops_at_full_health_and_restarts_after_damage() {
    let mut app = simulation();
    let player = spawn_player(&mut app, 99);

    for _ in 0..60 {
        app.update();
    }
    assert_eq!(health(&app, player), PlayerHealth::FULL);

    for _ in 0..30 {
        app.update();
    }
    app.world_mut().entity_mut(player).insert(PlayerHealth(98));
    for _ in 0..59 {
        app.update();
    }
    assert_eq!(health(&app, player), 98);
    app.update();
    assert_eq!(health(&app, player), 99);
}

#[test]
fn destroyed_players_do_not_regenerate() {
    let mut app = simulation();
    let player = spawn_player(&mut app, 0);

    for _ in 0..120 {
        app.update();
    }
    assert_eq!(health(&app, player), 0);
}
