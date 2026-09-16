use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use lightyear::prelude::{ReplicationMetadata, input::native::ActionState, server::ServerPlugins};

use project_game::{GamePlugin, PlayerBundle, SERVER_UPS, ServerSimulationPlugin};
use project_protocol::{AbilityCharge, PlayerInput, PlayerPhaseBeam, ProtocolPlugin};

fn simulation() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        TransformPlugin,
        ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
        },
        ProtocolPlugin,
        GamePlugin,
        ServerSimulationPlugin,
    ));
    app.insert_resource(ReplicationMetadata::new(Duration::from_secs_f64(
        1.0 / SERVER_UPS,
    )));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / SERVER_UPS,
    )));
    app.init_resource::<lightyear::connection::client::PeerMetadata>();
    app.finish();
    app.cleanup();
    app.update();
    app
}

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
