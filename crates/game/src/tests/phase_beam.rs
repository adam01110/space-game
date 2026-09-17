use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;

use space_game_protocol::{AbilityCharge, PlayerInput, PlayerPhaseBeam};

use crate::PlayerBundle;

use super::support::simulation;

#[test]
fn phase_beam_recharges_after_release() {
    let mut app = simulation();
    let mut input = ActionState::<PlayerInput>::default();
    input.0.phase_beam = true;
    let player = app
        .world_mut()
        .spawn((PlayerBundle::new(Vec2::ZERO), input))
        .id();

    for _ in 0..240 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerPhaseBeam>(player)
            .expect("phase beam charge")
            .0
            .units(),
        0
    );

    app.world_mut()
        .get_mut::<ActionState<PlayerInput>>(player)
        .expect("player input")
        .0
        .phase_beam = false;
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerPhaseBeam>(player)
            .expect("phase beam charge")
            .0
            .units(),
        25
    );

    for _ in 0..180 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerPhaseBeam>(player)
            .expect("phase beam charge")
            .0
            .units(),
        AbilityCharge::FULL
    );
}
