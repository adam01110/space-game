use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};

use project_protocol::{Player, PlayerHeading, PlayerInput, PlayerPosition};

const MOVE_SPEED: f32 = 512.0;
const TURN_SPEED: f32 = 8.0;

type PlayerMovement<'a> = (
    &'a mut PlayerPosition,
    &'a mut PlayerHeading,
    &'a ActionState<PlayerInput>,
);
type PredictedPlayer = (With<Player>, With<Predicted>);

// Runs prediction only after Lightyear has synchronized the client timeline.
pub(super) fn move_predicted_players(
    _timeline: SyncedLocalTimeline,
    time: Res<Time<Fixed>>,
    mut players: Query<PlayerMovement, PredictedPlayer>,
) {
    move_players(&mut players, time.delta_secs());
}

pub(super) fn move_authoritative_players(
    time: Res<Time<Fixed>>,
    mut players: Query<PlayerMovement, With<Player>>,
) {
    move_players(&mut players, time.delta_secs());
}

fn move_players<F: QueryFilter>(players: &mut Query<PlayerMovement, F>, delta_seconds: f32) {
    for (position, heading, input) in players {
        apply_movement(position, heading, input, delta_seconds);
    }
}

// Shared simulation used by the authoritative server and predicted clients.
fn apply_movement(
    mut position: Mut<PlayerPosition>,
    mut heading: Mut<PlayerHeading>,
    input: &ActionState<PlayerInput>,
    delta_seconds: f32,
) {
    let input = &input.0;

    if let Some(aim) = input.aim.try_normalize() {
        let target = aim.y.atan2(aim.x) - std::f32::consts::FRAC_PI_2;
        heading.0 = turn_towards(heading.0, target, TURN_SPEED * delta_seconds);
    }

    let velocity = input.movement.clamp_length_max(1.0) * MOVE_SPEED;
    position.0 += velocity * delta_seconds;
}

fn turn_towards(current: f32, target: f32, max_step: f32) -> f32 {
    let difference = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;

    current + difference.clamp(-max_step, max_step)
}
