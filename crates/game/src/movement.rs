use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::{Predicted, SyncedLocalTimeline};

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
    for (mut position, mut heading, input) in players {
        apply_movement(&mut position, &mut heading, &input.0, delta_seconds);
    }
}

// Shared simulation used by the authoritative server and predicted clients.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_movement_does_not_change_position() {
        for movement in [
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(f32::INFINITY, 0.0),
            Vec2::new(0.0, f32::NEG_INFINITY),
        ] {
            let mut position = PlayerPosition(Vec2::new(10.0, 20.0));
            let mut heading = PlayerHeading::default();
            let input = PlayerInput {
                movement,
                aim: Vec2::ZERO,
            };

            apply_movement(&mut position, &mut heading, &input, 1.0);

            assert_eq!(position.0, Vec2::new(10.0, 20.0));
        }
    }

    #[test]
    fn non_finite_aim_does_not_change_heading() {
        for aim in [
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(f32::INFINITY, 0.0),
            Vec2::new(0.0, f32::NEG_INFINITY),
        ] {
            let mut position = PlayerPosition::default();
            let mut heading = PlayerHeading(0.75);
            let input = PlayerInput {
                movement: Vec2::ZERO,
                aim,
            };

            apply_movement(&mut position, &mut heading, &input, 1.0);

            assert_eq!(heading.0, 0.75);
        }
    }

    #[test]
    fn finite_movement_is_normalized_and_applied() {
        let mut position = PlayerPosition(Vec2::new(10.0, 20.0));
        let mut heading = PlayerHeading::default();
        let movement = Vec2::new(3.0, 4.0);
        let input = PlayerInput {
            movement,
            aim: Vec2::ZERO,
        };

        apply_movement(&mut position, &mut heading, &input, 0.5);

        let expected = Vec2::new(10.0, 20.0) + movement.normalize() * MOVE_SPEED * 0.5;
        assert!(position.0.distance(expected) < f32::EPSILON);
    }
}
