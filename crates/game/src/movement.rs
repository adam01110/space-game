use avian2d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::{Predicted, SyncedLocalTimeline};

use space_game_protocol::{Player, PlayerInput, PlayerPhaseBeam};

const MOVE_SPEED: f32 = 512.0;
const PHASE_BEAM_MOVE_SPEED: f32 = MOVE_SPEED * 0.75;
const TURN_SPEED: f32 = 8.0;
const PHASE_BEAM_TURN_SPEED: f32 = 2.0;

// Snap aim within roughly one degree of an axis
const CARDINAL_SNAP_COMPONENT: f32 = 0.02;

type PlayerMovement<'a> = (
    &'a mut LinearVelocity,
    &'a mut Rotation,
    &'a ActionState<PlayerInput>,
    &'a PlayerPhaseBeam,
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
    for (mut velocity, mut rotation, input, phase_beam) in players {
        let phase_beam_active = input.0.phase_beam && phase_beam.0.units() > 0;

        apply_movement(
            &mut velocity,
            &mut rotation,
            &input.0,
            phase_beam_active,
            delta_seconds,
        );
    }
}

fn apply_movement(
    velocity: &mut LinearVelocity,
    rotation: &mut Rotation,
    input: &PlayerInput,
    phase_beam_active: bool,
    delta_seconds: f32,
) {
    apply_aim(rotation, input.aim, phase_beam_active, delta_seconds);

    // Never translate the body directly: the solver integrates velocity and blocks/slides
    // it at contacts. Invalid or released input must clear the previous desired velocity.
    velocity.0 = desired_velocity(input.movement, phase_beam_active);
}

fn apply_aim(rotation: &mut Rotation, aim: Vec2, phase_beam_active: bool, delta_seconds: f32) {
    let Some(aim) = normalized_aim(aim) else {
        return;
    };

    let target = aim.y.atan2(aim.x) - std::f32::consts::FRAC_PI_2;
    let step = turn_speed(phase_beam_active) * delta_seconds;

    *rotation = Rotation::radians(turn_towards(rotation.as_radians(), target, step));
}

fn normalized_aim(aim: Vec2) -> Option<Vec2> {
    aim.is_finite()
        .then_some(aim)
        .and_then(Vec2::try_normalize)
        .map(snap_cardinal_aim)
}

const fn turn_speed(phase_beam_active: bool) -> f32 {
    match phase_beam_active {
        true => PHASE_BEAM_TURN_SPEED,
        false => TURN_SPEED,
    }
}

const fn move_speed(phase_beam_active: bool) -> f32 {
    match phase_beam_active {
        true => PHASE_BEAM_MOVE_SPEED,
        false => MOVE_SPEED,
    }
}

fn desired_velocity(movement: Vec2, phase_beam_active: bool) -> Vec2 {
    match movement.is_finite() {
        true => movement.clamp_length_max(1.0) * move_speed(phase_beam_active),
        false => Vec2::ZERO,
    }
}

fn snap_cardinal_aim(aim: Vec2) -> Vec2 {
    match aim.x.abs() < CARDINAL_SNAP_COMPONENT {
        true => Vec2::new(0.0, aim.y.signum()),
        false => match aim.y.abs() < CARDINAL_SNAP_COMPONENT {
            true => Vec2::new(aim.x.signum(), 0.0),
            false => aim,
        },
    }
}

fn turn_towards(current: f32, target: f32, max_step: f32) -> f32 {
    let difference = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;

    current + difference.clamp(-max_step, max_step)
}
