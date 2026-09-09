use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::{Predicted, SyncedLocalTimeline};

use project_protocol::{Player, PlayerHeading, PlayerInput, PlayerPosition};

const MOVE_SPEED: f32 = 512.0;
const TURN_SPEED: f32 = 8.0;

// Both movement systems update position and heading from the player's latest input.
type PlayerMovement<'a> = (
    &'a mut PlayerPosition,
    &'a mut PlayerHeading,
    &'a ActionState<PlayerInput>,
);

type PredictedPlayer = (With<Player>, With<Predicted>);

// SyncedLocalTimeline prevents movement before Lightyear's local fixed clock is ready.
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
    for (mut position, mut heading, input) in players {
        apply_movement(&mut position, &mut heading, &input.0, delta_seconds);
    }
}

// Server and client movement use the same speed, turning, and input validation rules.
fn apply_movement(
    position: &mut PlayerPosition,
    heading: &mut PlayerHeading,
    input: &PlayerInput,
    delta_seconds: f32,
) {
    let normalized_aim = input
        .aim
        .is_finite()
        .then_some(input.aim)
        .and_then(Vec2::try_normalize);
    if let Some(aim) = normalized_aim {
        let target = aim.y.atan2(aim.x) - std::f32::consts::FRAC_PI_2;
        heading.0 = turn_towards(heading.0, target, TURN_SPEED * delta_seconds);
    }

    if input.movement.is_finite() {
        let velocity = input.movement.clamp_length_max(1.0) * MOVE_SPEED;
        position.0 += velocity * delta_seconds;
    }
}

fn turn_towards(current: f32, target: f32, max_step: f32) -> f32 {
    let difference = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;

    current + difference.clamp(-max_step, max_step)
}
