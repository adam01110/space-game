use bevy::prelude::*;
use space_game_protocol::PlayerHealth;

use crate::{HEALTH_REGEN_PER_SECOND, PlayerBundle};

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
fn damaged_players_regenerate_at_the_configured_rate_independently() {
    let mut app = simulation();
    let first = spawn_player(&mut app, 80);

    for _ in 0..60 {
        app.update();
    }
    assert_eq!(health(&app, first), 80 + HEALTH_REGEN_PER_SECOND);

    let second = spawn_player(&mut app, 70);
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(health(&app, first), 80 + 2 * HEALTH_REGEN_PER_SECOND);
    assert_eq!(health(&app, second), 70 + HEALTH_REGEN_PER_SECOND);
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
    app.update();
    assert_eq!(health(&app, player), 98);

    for _ in 0..59 {
        app.update();
    }
    assert_eq!(health(&app, player), PlayerHealth::FULL);
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
