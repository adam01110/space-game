use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};
use std::time::Duration;

use crate::PLAYER_RADIUS;
use project_protocol::{
    AbilityCharge, BlasterReload, BlasterShot, BlasterTrigger, Player, PlayerBlasters, PlayerInput,
};

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
// Consume inputs during reload without queuing shots or restarting the timer.
fn tick_reload(charge: &mut PlayerBlasters, reload: &mut BlasterReload, delta: Duration) {
    reload.remaining = reload.remaining.saturating_sub(delta);

    if reload.remaining.is_zero() {
        charge.0 = AbilityCharge::default();
    }
}

// A reload starts when the blaster cannot fire but should be ready soon.
fn wants_reload(charge: &PlayerBlasters, reload_requested: bool) -> bool {
    let empty = charge.0.units() < SHOT_COST;
    let manual = reload_requested && charge.0.units() < AbilityCharge::FULL;

    empty || manual
}

fn update_blasters(
    charge: &mut PlayerBlasters,
    trigger: &mut BlasterTrigger,
    reload: &mut BlasterReload,
    input: &PlayerInput,
    delta: Duration,
) -> u16 {
    let clicks = consume_counter(&mut trigger.0, input.blaster_clicks);
    let reload_requested = consume_counter(&mut reload.requests, input.blaster_reload_requests) > 0;

    if !reload.remaining.is_zero() {
        tick_reload(charge, reload, delta);
        return 0;
    }

    if wants_reload(charge, reload_requested) {
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

fn spawn_shots(commands: &mut Commands, position: &Position, rotation: &Rotation, shots: u16) {
    let direction = *rotation * Vec2::Y;

    for _ in 0..shots {
        commands.spawn(BlasterShot {
            position: position.0 + direction * (PLAYER_RADIUS + 8.0),
            direction,
            ticks_left: SHOT_LIFETIME,
        });
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
        spawn_shots(&mut commands, position, rotation, shots);
    }
}

fn advance_shot(
    commands: &mut Commands,
    shot_entity: Entity,
    shot: &mut BlasterShot,
    delta_secs: f32,
) {
    if shot.ticks_left <= 1 {
        // The caller must not use the entity afterwards.
        commands.entity(shot_entity).despawn();
    } else {
        shot.ticks_left -= 1;
        shot.position += shot.direction * SHOT_SPEED * delta_secs;
    }
}

pub(super) fn advance_shots(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut shots: Query<(Entity, &mut BlasterShot)>,
) {
    let delta_secs = time.delta_secs();

    for (shot_entity, mut shot) in &mut shots {
        advance_shot(&mut commands, shot_entity, &mut shot, delta_secs);
    }
}
