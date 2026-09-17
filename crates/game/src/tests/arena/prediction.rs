use avian2d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use lightyear::prelude::{FrameInterpolate, Predicted};

use project_protocol::{ArenaBoundary, CircleBody};

use crate::{contain_predicted_bodies, install_arena_interpolation};

#[test]
fn render_interpolation_does_not_change_next_simulation_radius() {
    use lightyear::{
        frame_interpolation::FrameInterpolationPlugin, prelude::FrameInterpolationHistory,
    };
    let mut app = App::new();
    app.insert_resource(Time::<Fixed>::from_seconds(1.0));
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .accumulate_overstep(std::time::Duration::from_millis(250));
    app.add_plugins(FrameInterpolationPlugin);
    install_arena_interpolation(&mut app);
    let entity = app
        .world_mut()
        .spawn((
            ArenaBoundary::new(1004.0),
            FrameInterpolate,
            FrameInterpolationHistory::<ArenaBoundary> {
                previous_value: Some(ArenaBoundary::new(1000.0)),
                current_value: Some(ArenaBoundary::new(1004.0)),
            },
        ))
        .id();
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(
        app.world()
            .get::<ArenaBoundary>(entity)
            .expect("render radius")
            .radius,
        1001.0
    );
    app.world_mut().run_schedule(RunFixedMainLoop);
    assert_eq!(
        app.world()
            .get::<ArenaBoundary>(entity)
            .expect("simulation radius")
            .radius,
        1004.0
    );
}

#[test]
fn prediction_uses_predicted_radius_and_ignores_confirmed_bodies() {
    let mut app = App::new();
    app.add_systems(Update, contain_predicted_bodies);
    app.world_mut().spawn(ArenaBoundary::new(2000.0));
    let arena = app
        .world_mut()
        .spawn((ArenaBoundary::new(768.0), Predicted))
        .id();
    let predicted = app
        .world_mut()
        .spawn((
            CircleBody::dynamic(23.4),
            Position(Vec2::X * 3000.0),
            LinearVelocity(Vec2::X * 10_000.0),
            Predicted,
        ))
        .id();
    let confirmed = app
        .world_mut()
        .spawn((
            CircleBody::dynamic(23.4),
            Position(Vec2::X * 3000.0),
            LinearVelocity(Vec2::X * 10_000.0),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<Position>(predicted)
            .expect("predicted pose")
            .length()
            < 745.0
    );
    assert_eq!(
        app.world()
            .get::<Position>(confirmed)
            .expect("confirmed pose")
            .x,
        3000.0
    );
    // A corrected/rolled-back radius must immediately drive the same constraint.
    app.world_mut()
        .get_mut::<ArenaBoundary>(arena)
        .expect("arena")
        .radius = 400.0;
    app.update();
    assert!(
        app.world()
            .get::<Position>(predicted)
            .expect("predicted pose")
            .length()
            < 377.0
    );
}
