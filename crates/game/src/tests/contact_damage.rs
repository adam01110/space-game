use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;

use space_game_protocol::{Asteroid, AsteroidHealth, CircleBody, PlayerHealth};

use crate::{CONTACT_DAMAGE, PlayerBundle};

use super::support::simulation;

#[test]
fn contact_damages_both_once_until_they_separate() {
    let mut app = simulation();
    let player = app.world_mut().spawn(PlayerBundle::new(Vec2::ZERO)).id();
    let asteroid = app
        .world_mut()
        .spawn((
            Asteroid {
                variant: 0,
                radius: 35.0,
                stretch: Vec2::ONE,
                angle: 0.0,
            },
            AsteroidHealth(100),
            CircleBody::fixed(35.0),
            Position(Vec2::new(0.0, 55.0)),
            Rotation::default(),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<AsteroidHealth>(asteroid).unwrap().0,
        100 - CONTACT_DAMAGE
    );
    assert_eq!(
        app.world().get::<PlayerHealth>(player).unwrap().0,
        100 - CONTACT_DAMAGE
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world().get::<AsteroidHealth>(asteroid).unwrap().0,
        100 - CONTACT_DAMAGE
    );
}

#[test]
fn contact_overkill_is_equal_on_both_sides() {
    let mut app = simulation();
    let player = app.world_mut().spawn(PlayerBundle::new(Vec2::ZERO)).id();
    let asteroid = app
        .world_mut()
        .spawn((
            Asteroid {
                variant: 1,
                radius: 35.0,
                stretch: Vec2::ONE,
                angle: 0.0,
            },
            AsteroidHealth(3),
            CircleBody::fixed(35.0),
            Position(Vec2::new(0.0, 55.0)),
            Rotation::default(),
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<PlayerHealth>(player).unwrap().0, 97);
    // The destroyed asteroid is removed on the following authoritative damage pass.
    app.update();
    assert!(app.world().get_entity(asteroid).is_err());
}
