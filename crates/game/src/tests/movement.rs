use avian2d::prelude::{LinearVelocity, Rotation};
use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;

use space_game_protocol::{PlayerBoost, PlayerInput};

use crate::PlayerBundle;

use super::support::simulation;

fn spawn_turning_player(app: &mut App, phase_beam: bool) -> Entity {
    let mut input = ActionState::<PlayerInput>::default();
    input.0.movement = Vec2::X;
    input.0.aim = Vec2::X;
    input.0.phase_beam = phase_beam;

    let position = match phase_beam {
        true => Vec2::X * 100.0,
        false => Vec2::X * -100.0,
    };
    app.world_mut()
        .spawn((PlayerBundle::new(position), input))
        .id()
}

#[test]
fn phase_beam_reduces_movement_and_turn_speeds() {
    let mut app = simulation();
    let regular_player = spawn_turning_player(&mut app, false);
    let phase_beam_player = spawn_turning_player(&mut app, true);

    app.update();

    let regular_turn = app
        .world()
        .get::<Rotation>(regular_player)
        .expect("regular player rotation")
        .as_radians()
        .abs();
    let phase_beam_turn = app
        .world()
        .get::<Rotation>(phase_beam_player)
        .expect("phase beam player rotation")
        .as_radians()
        .abs();
    let regular_speed = app
        .world()
        .get::<LinearVelocity>(regular_player)
        .expect("regular player velocity")
        .length();
    let phase_beam_speed = app
        .world()
        .get::<LinearVelocity>(phase_beam_player)
        .expect("phase beam player velocity")
        .length();

    assert!(phase_beam_turn < regular_turn);
    assert!((phase_beam_turn * 4.0 - regular_turn).abs() < 0.001);
    assert!((phase_beam_speed - regular_speed * 0.75).abs() < 0.001);
}

#[test]
fn boost_only_activates_while_moving_forward() {
    let mut app = simulation();
    let player = spawn_turning_player(&mut app, false);

    for (movement, expected_speed) in [(Vec2::ZERO, 0.0), (Vec2::Y, 512.0), (Vec2::NEG_X, 512.0)] {
        let mut input = app
            .world_mut()
            .get_mut::<ActionState<PlayerInput>>(player)
            .expect("player input");
        input.0.boost = true;
        input.0.movement = movement;

        for _ in 0..60 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<PlayerBoost>(player)
                .expect("boost charge")
                .0
                .units(),
            50
        );
        assert!(
            (app.world()
                .get::<LinearVelocity>(player)
                .expect("player velocity")
                .length()
                - expected_speed)
                .abs()
                < 0.001
        );
    }

    app.world_mut()
        .get_mut::<ActionState<PlayerInput>>(player)
        .expect("player input")
        .0
        .movement = Vec2::X;
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerBoost>(player)
            .expect("boost charge")
            .0
            .units(),
        48
    );
    assert!(
        (app.world()
            .get::<LinearVelocity>(player)
            .expect("boost velocity")
            .length()
            - 512.0 * 1.5)
            .abs()
            < 0.001
    );
}

#[test]
fn boost_is_finite_and_multiplies_movement_speed() {
    let mut app = simulation();
    let player = spawn_turning_player(&mut app, false);
    let mut input = app
        .world_mut()
        .get_mut::<ActionState<PlayerInput>>(player)
        .expect("player input");
    input.0.boost = true;

    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerBoost>(player)
            .expect("boost charge")
            .0
            .units(),
        50
    );
    assert!(
        (app.world()
            .get::<LinearVelocity>(player)
            .expect("boost velocity")
            .length()
            - 512.0 * 1.5)
            .abs()
            < 0.001
    );

    for _ in 1..60 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerBoost>(player)
            .expect("boost charge")
            .0
            .units(),
        48
    );

    app.world_mut()
        .get_mut::<ActionState<PlayerInput>>(player)
        .expect("player input")
        .0
        .boost = false;
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerBoost>(player)
            .expect("boost charge")
            .0
            .units(),
        48
    );
    assert!(
        (app.world()
            .get::<LinearVelocity>(player)
            .expect("regular velocity")
            .length()
            - 512.0)
            .abs()
            < 0.001
    );

    app.world_mut()
        .get_mut::<ActionState<PlayerInput>>(player)
        .expect("player input")
        .0
        .boost = true;
    for _ in 0..1440 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerBoost>(player)
            .expect("boost charge")
            .0
            .units(),
        0
    );
    app.update();
    assert!(
        (app.world()
            .get::<LinearVelocity>(player)
            .expect("exhausted velocity")
            .length()
            - 512.0)
            .abs()
            < 0.001
    );
}
