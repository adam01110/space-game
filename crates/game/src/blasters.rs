use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};
use project_protocol::{
    AbilityCharge, BlasterShot, BlasterTrigger, Player, PlayerBlasters, PlayerInput,
};

use crate::PLAYER_RADIUS;

const SHOT_COST: u16 = 2 * AbilityCharge::UNITS_PER_PERCENT;
const REGEN_PER_SECOND: u16 = 2 * AbilityCharge::UNITS_PER_PERCENT;
const SHOT_SPEED: f32 = 1200.0;
const SHOT_LIFETIME: u16 = 60;

type BlasterState<'a> = (
    &'a mut PlayerBlasters,
    &'a mut BlasterTrigger,
    &'a ActionState<PlayerInput>,
);

// Counters make resending/holding the same input idempotent, including during rollback.
fn consume_clicks(charge: &mut PlayerBlasters, trigger: &mut BlasterTrigger, clicks: u32) -> u16 {
    let pending = clicks.wrapping_sub(trigger.0);

    // An older packet must not look like billions of new clicks.
    if pending > u32::MAX / 2 {
        return 0;
    }

    trigger.0 = clicks;
    charge.0.spend(SHOT_COST, pending)
}

pub(super) fn shoot_predicted_players(
    _timeline: SyncedLocalTimeline,
    time: Res<Time<Fixed>>,
    mut players: Query<BlasterState, (With<Player>, With<Predicted>)>,
) {
    for (mut charge, mut trigger, input) in &mut players {
        charge.0.regenerate(REGEN_PER_SECOND, time.delta());
        consume_clicks(&mut charge, &mut trigger, input.0.blaster_clicks);
    }
}

pub(super) fn shoot_authoritative_players(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut players: Query<(BlasterState, &Position, &Rotation), With<Player>>,
) {
    for ((mut charge, mut trigger, input), position, rotation) in &mut players {
        charge.0.regenerate(REGEN_PER_SECOND, time.delta());
        let shots = consume_clicks(&mut charge, &mut trigger, input.0.blaster_clicks);
        let direction = *rotation * Vec2::Y;

        for _ in 0..shots {
            commands.spawn(BlasterShot {
                position: position.0 + direction * (PLAYER_RADIUS + 8.0),
                direction,
                ticks_left: SHOT_LIFETIME,
            });
        }
    }
}

pub(super) fn advance_shots(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut shots: Query<(Entity, &mut BlasterShot)>,
) {
    for (entity, mut shot) in &mut shots {
        if shot.ticks_left <= 1 {
            commands.entity(entity).despawn();
        } else {
            let direction = shot.direction;

            shot.ticks_left -= 1;
            shot.position += direction * SHOT_SPEED * time.delta_secs();
        }
    }
}
