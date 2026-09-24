use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{
    PreSpawned, Predicted, PredictionDespawnCommandsExt, SyncedLocalTimeline,
};

use space_game_protocol::{BlasterShot, BlasterTrajectory, PlayerIdentity};

use crate::PLAYER_RADIUS;

const SHOT_SPEED: f32 = 1200.0;
const SHOT_LIFETIME: u8 = 60;

type PredictedShots<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut BlasterShot, &'static BlasterTrajectory),
    Or<(With<Predicted>, With<PreSpawned>)>,
>;

pub(super) fn spawn_shots(
    commands: &mut Commands,
    position: &Position,
    rotation: &Rotation,
    identity: PlayerIdentity,
    shots: u8,
    last_click: u8,
    owner: Option<Entity>,
) {
    let direction = *rotation * Vec2::Y;

    for shot_index in 0..shots {
        // A retransmitted click can be consumed on a different server tick, so match the
        // action, not the consumption tick. Counters cannot wrap within the one-second
        // lifetime: a magazine holds 50 shots then reloads for 5 s.
        let click = last_click.wrapping_sub(shots - 1 - shot_index);
        let hash = identity
            .0
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(17)
            ^ u64::from(click);
        let prespawn = PreSpawned::new(hash);
        commands.spawn((
            BlasterShot {
                position: position.0 + direction * (PLAYER_RADIUS + 8.0),
                ticks_left: SHOT_LIFETIME,
            },
            BlasterTrajectory { direction },
            // Only the controlling client prespawns. Other clients receive an
            // authoritative projectile and predict it on the player timeline.
            owner.map_or(prespawn, |owner| prespawn.for_client(owner)),
        ));
    }
}

pub(crate) fn advance_authoritative_shots(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut shots: Query<(Entity, &mut BlasterShot, &BlasterTrajectory)>,
) {
    for (entity, mut shot, trajectory) in &mut shots {
        if shot.ticks_left <= 1 {
            commands.entity(entity).despawn();
        } else {
            advance_shot(&mut shot, trajectory, time.delta_secs());
        }
    }
}

// Never simulate confirmed-only copies. Predicted expiry must retain history so
// a rollback can restore the projectile instead of losing it permanently.
pub(crate) fn advance_predicted_shots(
    _timeline: SyncedLocalTimeline,
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut shots: PredictedShots,
) {
    for (entity, mut shot, trajectory) in &mut shots {
        if shot.ticks_left <= 1 {
            commands.entity(entity).prediction_despawn();
        } else {
            advance_shot(&mut shot, trajectory, time.delta_secs());
        }
    }
}

fn advance_shot(shot: &mut BlasterShot, trajectory: &BlasterTrajectory, delta_secs: f32) {
    shot.ticks_left -= 1;
    shot.position += trajectory.direction * SHOT_SPEED * delta_secs;
}
