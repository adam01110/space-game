use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::PreSpawned;

use space_game_protocol::{BlasterShot, PlayerIdentity};

use crate::PLAYER_RADIUS;

const SHOT_SPEED: f32 = 1200.0;
const SHOT_LIFETIME: u8 = 60;

pub(super) fn spawn_shots(
    commands: &mut Commands,
    position: &Position,
    rotation: &Rotation,
    identity: PlayerIdentity,
    shots: u8,
) {
    let direction = *rotation * Vec2::Y;

    for shot_index in 0..shots {
        commands.spawn((
            BlasterShot {
                position: position.0 + direction * (PLAYER_RADIUS + 8.0),
                direction,
                ticks_left: SHOT_LIFETIME,
            },
            // The spawn tick plus this salt lets Lightyear match the immediate client shot to
            // the server copy instead of showing a second projectile one round trip later.
            PreSpawned::default_with_salt(identity.0 ^ u64::from(shot_index)),
        ));
    }
}

fn advance_shot(
    commands: &mut Commands,
    shot_entity: Entity,
    shot: &mut BlasterShot,
    delta_secs: f32,
) {
    match shot.ticks_left <= 1 {
        true => {
            // The caller must not use the entity afterwards.
            commands.entity(shot_entity).despawn();
        }
        false => {
            shot.ticks_left -= 1;
            shot.position += shot.direction * SHOT_SPEED * delta_secs;
        }
    }
}

pub(crate) fn advance_shots(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut shots: Query<(Entity, &mut BlasterShot)>,
) {
    let delta_secs = time.delta_secs();

    for (shot_entity, mut shot) in &mut shots {
        advance_shot(&mut commands, shot_entity, &mut shot, delta_secs);
    }
}
