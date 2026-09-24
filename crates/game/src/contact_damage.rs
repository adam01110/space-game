use std::collections::HashSet;

use avian2d::prelude::Collisions;
use bevy::prelude::*;

use space_game_protocol::{Asteroid, AsteroidHealth, Player, PlayerHealth};

pub const CONTACT_DAMAGE: u8 = 10;

// Server-authoritative contact damage. A sustained collision counts once; separating and
// touching again counts as a new impact. Both sides lose the same actual (non-overkill) amount.
pub(super) fn damage_on_asteroid_contact(
    collisions: Collisions,
    mut players: Query<&mut PlayerHealth, With<Player>>,
    mut asteroids: Query<&mut AsteroidHealth, With<Asteroid>>,
    mut previous: Local<HashSet<(Entity, Entity)>>,
) {
    let mut touching = HashSet::new();
    for pair in collisions.iter() {
        let (player, asteroid) =
            if players.contains(pair.collider1) && asteroids.contains(pair.collider2) {
                (pair.collider1, pair.collider2)
            } else if players.contains(pair.collider2) && asteroids.contains(pair.collider1) {
                (pair.collider2, pair.collider1)
            } else {
                continue;
            };
        touching.insert((player, asteroid));
        if previous.contains(&(player, asteroid)) {
            continue;
        }
        let (Ok(mut player_health), Ok(mut asteroid_health)) =
            (players.get_mut(player), asteroids.get_mut(asteroid))
        else {
            continue;
        };
        if player_health.0 == 0 {
            continue;
        }
        let damage = asteroid_health.0.min(CONTACT_DAMAGE);
        asteroid_health.0 -= damage;
        player_health.0 = player_health.0.saturating_sub(damage);
    }
    *previous = touching;
}
