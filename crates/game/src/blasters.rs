mod shots;

use std::time::Duration;

use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};

use project_protocol::{
    AbilityCharge, BlasterReload, BlasterTrigger, Player, PlayerBlasters, PlayerIdentity,
    PlayerInput,
};

pub(crate) use shots::advance_shots;
use shots::spawn_shots;

const SHOT_COST: u8 = 2;
const RELOAD_DURATION: Duration = Duration::from_secs(5);

// Only one of these applies per tick, in this order: a running reload, a wanted reload, firing.
enum BlasterAction {
    TickReload,
    StartReload,
    Fire,
}

type BlasterState<'a> = (
    &'a mut PlayerBlasters,
    &'a mut BlasterTrigger,
    &'a mut BlasterReload,
    &'a ActionState<PlayerInput>,
);

type FiringPlayer<'a> = (
    BlasterState<'a>,
    &'a Position,
    &'a Rotation,
    &'a PlayerIdentity,
);

// Counters make repeated input idempotent, including during rollback.
fn consume_counter(previous: &mut u8, current: u8) -> u8 {
    let pending = current.wrapping_sub(*previous);

    // An older packet must not look like a burst of new presses.
    if pending > u8::MAX / 2 {
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

fn pending_action(
    charge: &PlayerBlasters,
    reload: &BlasterReload,
    reload_requested: bool,
) -> BlasterAction {
    if !reload.remaining.is_zero() {
        BlasterAction::TickReload
    } else if wants_reload(charge, reload_requested) {
        BlasterAction::StartReload
    } else {
        BlasterAction::Fire
    }
}

fn update_blasters(
    charge: &mut PlayerBlasters,
    trigger: &mut BlasterTrigger,
    reload: &mut BlasterReload,
    input: &PlayerInput,
    delta: Duration,
) -> u8 {
    let clicks = consume_counter(&mut trigger.0, input.blaster_clicks);
    let reload_requested = consume_counter(&mut reload.requests, input.blaster_reload_requests) > 0;

    match pending_action(charge, reload, reload_requested) {
        BlasterAction::TickReload => {
            tick_reload(charge, reload, delta);
            0
        }
        BlasterAction::StartReload => {
            reload.remaining = RELOAD_DURATION;
            0
        }
        BlasterAction::Fire => {
            let shots = charge.0.spend(SHOT_COST, clicks);
            // An emptied blaster reloads instead of firing again.
            if charge.0.units() < SHOT_COST {
                reload.remaining = RELOAD_DURATION;
            }
            shots
        }
    }
}

pub(super) fn shoot_predicted_players(
    _timeline: SyncedLocalTimeline,
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut players: Query<FiringPlayer, (With<Player>, With<Predicted>)>,
) {
    for ((mut charge, mut trigger, mut reload, input), position, rotation, identity) in &mut players
    {
        let shots = update_blasters(
            &mut charge,
            &mut trigger,
            &mut reload,
            &input.0,
            time.delta(),
        );

        spawn_shots(&mut commands, position, rotation, *identity, shots);
    }
}

pub(super) fn shoot_authoritative_players(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut players: Query<FiringPlayer, With<Player>>,
) {
    for ((mut charge, mut trigger, mut reload, input), position, rotation, identity) in &mut players
    {
        let shots = update_blasters(
            &mut charge,
            &mut trigger,
            &mut reload,
            &input.0,
            time.delta(),
        );
        spawn_shots(&mut commands, position, rotation, *identity, shots);
    }
}
