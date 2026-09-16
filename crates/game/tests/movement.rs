use std::time::Duration;

use avian2d::prelude::{LinearVelocity, Rotation};
use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use lightyear::prelude::{ReplicationMetadata, input::native::ActionState, server::ServerPlugins};

use project_game::{GamePlugin, PlayerBundle, SERVER_UPS, ServerSimulationPlugin};
use project_protocol::{PlayerInput, ProtocolPlugin};

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
