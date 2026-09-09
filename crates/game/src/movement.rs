use avian2d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::{Predicted, SyncedLocalTimeline};

use project_protocol::{Player, PlayerInput};

const MOVE_SPEED: f32 = 512.0;
const TURN_SPEED: f32 = 8.0;

type PlayerMovement<'a> = (
    &'a mut LinearVelocity,
    &'a mut Rotation,
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
    for (mut velocity, mut rotation, input) in players {
        apply_movement(&mut velocity, &mut rotation, &input.0, delta_seconds);
    }
}

fn apply_movement(
    velocity: &mut LinearVelocity,
    rotation: &mut Rotation,
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
        *rotation = Rotation::radians(turn_towards(
            rotation.as_radians(),
            target,
            TURN_SPEED * delta_seconds,
        ));
    }

    // Never translate the body directly: the solver integrates velocity and blocks/slides
    // it at contacts. Invalid or released input must clear the previous desired velocity.
    velocity.0 = if input.movement.is_finite() {
        input.movement.clamp_length_max(1.0) * MOVE_SPEED
    } else {
        Vec2::ZERO
    };
}

fn turn_towards(current: f32, target: f32, max_step: f32) -> f32 {
    let difference = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;

    current + difference.clamp(-max_step, max_step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_movement_clears_velocity() {
        for movement in [
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(f32::INFINITY, 0.0),
            Vec2::new(0.0, f32::NEG_INFINITY),
        ] {
            let mut velocity = LinearVelocity(Vec2::splat(MOVE_SPEED));
            let mut rotation = Rotation::default();
            let input = PlayerInput {
                movement,
                aim: Vec2::ZERO,
            };
            apply_movement(&mut velocity, &mut rotation, &input, 1.0);
            assert_eq!(velocity.0, Vec2::ZERO);
        }
    }

    #[test]
    fn non_finite_aim_does_not_change_rotation() {
        for aim in [
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(f32::INFINITY, 0.0),
            Vec2::new(0.0, f32::NEG_INFINITY),
        ] {
            let mut velocity = LinearVelocity::default();
            let mut rotation = Rotation::radians(0.75);
            let input = PlayerInput {
                movement: Vec2::ZERO,
                aim,
            };
            apply_movement(&mut velocity, &mut rotation, &input, 1.0);
            assert_eq!(rotation, Rotation::radians(0.75));
        }
    }

    #[test]
    fn movement_sets_normalized_velocity_and_release_stops() {
        let mut velocity = LinearVelocity::default();
        let mut rotation = Rotation::default();
        let movement = Vec2::new(3.0, 4.0);
        let input = PlayerInput {
            movement,
            aim: Vec2::ZERO,
        };
        apply_movement(&mut velocity, &mut rotation, &input, 0.5);
        assert!(velocity.0.distance(movement.normalize() * MOVE_SPEED) < f32::EPSILON);
        apply_movement(&mut velocity, &mut rotation, &PlayerInput::default(), 0.5);
        assert_eq!(velocity.0, Vec2::ZERO);
    }
}
