use bevy::prelude::*;
use lightyear::prelude::{ControlledBy, input::native::ActionState};

use space_game_protocol::{BlasterShot, BlasterTrajectory, PlayerHealth, PlayerInput};

use crate::{BULLET_DAMAGE, DamageConfig, PHASE_BEAM_DAMAGE_PER_SECOND, PlayerBundle};

use super::support::simulation;

const FULL_HEALTH: u8 = 100;

// The server tags players and their shots with the owning connection, so a player's own weapons
// stay harmless.
fn owner(app: &mut App) -> Entity {
    app.world_mut().spawn_empty().id()
}

fn owned_by(app: &mut App, position: Vec2, owner: Entity) -> Entity {
    app.world_mut()
        .spawn((
            PlayerBundle::new(position),
            ControlledBy {
                owner,
                lifetime: default(),
            },
        ))
        .id()
}

// A player with a connection of its own, for targets nothing shoots back with.
fn target(app: &mut App, position: Vec2) -> Entity {
    let owner = owner(app);

    owned_by(app, position, owner)
}

// A shot 400 units below the target, travelling straight at it one 20-unit step per tick.
fn shot_towards(app: &mut App, position: Vec2, owner: Option<Entity>) -> Entity {
    let shot = (
        BlasterShot {
            position: position - Vec2::Y * 400.0,
            ticks_left: 60,
        },
        BlasterTrajectory { direction: Vec2::Y },
    );

    match owner {
        Some(owner) => app
            .world_mut()
            .spawn((
                shot,
                ControlledBy {
                    owner,
                    lifetime: default(),
                },
            ))
            .id(),
        None => app.world_mut().spawn(shot).id(),
    }
}

fn held_beam() -> ActionState<PlayerInput> {
    let mut input = ActionState::<PlayerInput>::default();
    input.0.phase_beam = true;
    input
}

fn health(app: &App, player: Entity) -> u8 {
    app.world()
        .get::<PlayerHealth>(player)
        .expect("player health")
        .0
}

#[test]
fn blaster_shot_damages_the_player_it_hits() {
    let mut app = simulation();
    let shooter = owner(&mut app);
    let victim = target(&mut app, Vec2::ZERO);
    let shot = shot_towards(&mut app, Vec2::ZERO, Some(shooter));

    for _ in 0..20 {
        app.update();
    }

    assert_eq!(health(&app, victim), FULL_HEALTH - BULLET_DAMAGE);
    assert!(
        app.world().get_entity(shot).is_err(),
        "the hit consumes the shot"
    );
}

#[test]
fn blaster_shot_passing_beside_the_player_does_not_damage_it() {
    let mut app = simulation();
    let victim = target(&mut app, Vec2::ZERO);
    app.world_mut().spawn((
        BlasterShot {
            position: Vec2::new(100.0, -400.0),
            ticks_left: 60,
        },
        BlasterTrajectory { direction: Vec2::Y },
    ));

    for _ in 0..60 {
        app.update();
    }

    assert_eq!(health(&app, victim), FULL_HEALTH);
}

#[test]
fn phase_beam_damage_matches_its_per_second_rate() {
    let mut app = simulation();
    let attacker_owner = owner(&mut app);
    let attacker = owned_by(&mut app, Vec2::ZERO, attacker_owner);
    app.world_mut().entity_mut(attacker).insert(held_beam());
    let victim = target(&mut app, Vec2::new(0.0, 300.0));

    // Two seconds of contact at the configured rate; the fixed clock can advance an extra step at a
    // boundary, so allow the one damage unit that comes with it.
    for _ in 0..120 {
        app.update();
    }

    let damage = FULL_HEALTH - health(&app, victim);
    let expected = PHASE_BEAM_DAMAGE_PER_SECOND * 2;
    assert!(
        damage.abs_diff(expected) <= 1,
        "expected about {expected} damage after two seconds, got {damage}"
    );
    // The beam leaves its own muzzle, which touches the shooter's hit circle exactly.
    assert_eq!(health(&app, attacker), FULL_HEALTH);
}

#[test]
fn damage_config_overrides_the_defaults_without_wrapping_health() {
    let mut app = simulation();
    app.insert_resource(DamageConfig {
        bullet: u8::MAX,
        phase_beam_per_second: 0,
    });
    let victim = target(&mut app, Vec2::ZERO);
    shot_towards(&mut app, Vec2::ZERO, None);

    for _ in 0..20 {
        app.update();
    }

    assert_eq!(health(&app, victim), 0);
}
