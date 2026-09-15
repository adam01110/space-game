use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};
use project_protocol::{
    AbilityCharge, BlasterReload, BlasterShot, BlasterTrigger, Player, PlayerBlasters, PlayerInput,
};
use std::time::Duration;

use crate::PLAYER_RADIUS;

const SHOT_COST: u16 = 2 * AbilityCharge::UNITS_PER_PERCENT;
const RELOAD_DURATION: Duration = Duration::from_secs(5);
const SHOT_SPEED: f32 = 1200.0;
const SHOT_LIFETIME: u16 = 60;

type BlasterState<'a> = (
    &'a mut PlayerBlasters,
    &'a mut BlasterTrigger,
    &'a mut BlasterReload,
    &'a ActionState<PlayerInput>,
);

// Counters make repeated input idempotent, including during rollback.
fn consume_counter(previous: &mut u32, current: u32) -> u32 {
    let pending = current.wrapping_sub(*previous);

    // An older packet must not look like billions of new presses.
    if pending > u32::MAX / 2 {
        return 0;
    }

    *previous = current;
    pending
}

// Both prediction and authority use the same reload and firing rules.
fn update_blasters(
    charge: &mut PlayerBlasters,
    trigger: &mut BlasterTrigger,
    reload: &mut BlasterReload,
    input: &PlayerInput,
    delta: Duration,
) -> u16 {
    let clicks = consume_counter(&mut trigger.0, input.blaster_clicks);
    let reload_requested = consume_counter(&mut reload.requests, input.blaster_reload_requests) > 0;

    // Consume inputs during reload without queuing shots or restarting the timer.
    if !reload.remaining.is_zero() {
        reload.remaining = reload.remaining.saturating_sub(delta);
        if reload.remaining.is_zero() {
            charge.0 = AbilityCharge::default();
        }
        return 0;
    }

    if charge.0.units() < SHOT_COST || (reload_requested && charge.0.units() < AbilityCharge::FULL)
    {
        reload.remaining = RELOAD_DURATION;
        return 0;
    }

    let shots = charge.0.spend(SHOT_COST, clicks);
    if charge.0.units() < SHOT_COST {
        reload.remaining = RELOAD_DURATION;
    }
    shots
}

pub(super) fn shoot_predicted_players(
    _timeline: SyncedLocalTimeline,
    time: Res<Time<Fixed>>,
    mut players: Query<BlasterState, (With<Player>, With<Predicted>)>,
) {
    for (mut charge, mut trigger, mut reload, input) in &mut players {
        update_blasters(
            &mut charge,
            &mut trigger,
            &mut reload,
            &input.0,
            time.delta(),
        );
    }
}

pub(super) fn shoot_authoritative_players(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut players: Query<(BlasterState, &Position, &Rotation), With<Player>>,
) {
    for ((mut charge, mut trigger, mut reload, input), position, rotation) in &mut players {
        let shots = update_blasters(
            &mut charge,
            &mut trigger,
            &mut reload,
            &input.0,
            time.delta(),
        );
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
