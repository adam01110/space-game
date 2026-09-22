use bevy::prelude::*;
use lightyear::prelude::{
    LocalTimeline, LocalTimelineSync, Predicted, PredictionManager, input::native::ActionState,
};
use space_game_protocol::{
    BlasterReload, BlasterShot, BlasterTrajectory, BlasterTrigger, PlayerBlasters, PlayerIdentity,
    PlayerInput, PlayerPhaseBeam,
};

use super::support::simulation;
use crate::{PlayerBundle, blasters::advance_predicted_shots};

#[test]
fn idle_weapons_do_not_dirty_replicated_components() {
    let mut app = simulation();
    let player = app
        .world_mut()
        .spawn((
            PlayerBundle::new(Vec2::ZERO),
            PlayerIdentity(42),
            ActionState::<PlayerInput>::default(),
        ))
        .id();
    app.update();
    app.world_mut().clear_trackers();
    app.update();
    let entity = app.world().entity(player);
    assert!(
        !entity
            .get_ref::<PlayerBlasters>()
            .expect("charge")
            .is_changed()
    );
    assert!(
        !entity
            .get_ref::<BlasterTrigger>()
            .expect("trigger")
            .is_changed()
    );
    assert!(
        !entity
            .get_ref::<BlasterReload>()
            .expect("reload")
            .is_changed()
    );
    assert!(
        !entity
            .get_ref::<PlayerPhaseBeam>()
            .expect("beam charge")
            .is_changed()
    );
}

#[test]
fn expiry_retains_predicted_history_and_ignores_confirmed_copies() {
    let mut app = App::new();
    app.init_resource::<Time<Fixed>>();
    app.init_resource::<LocalTimeline>();
    app.init_resource::<LocalTimelineSync>();
    app.init_resource::<PredictionManager>();
    app.world_mut()
        .resource_mut::<LocalTimelineSync>()
        .set_synced(true);
    app.add_systems(Update, advance_predicted_shots);
    let shot = BlasterShot {
        position: Vec2::ZERO,
        ticks_left: 1,
    };
    let trajectory = BlasterTrajectory { direction: Vec2::Y };
    let predicted = app.world_mut().spawn((shot, trajectory, Predicted)).id();
    let confirmed = app.world_mut().spawn((shot, trajectory)).id();
    app.update();
    assert!(
        app.world()
            .get::<lightyear::prediction::despawn::PredictionDisable>(predicted)
            .is_some()
    );
    assert_eq!(app.world().get::<BlasterShot>(predicted), Some(&shot));
    assert_eq!(app.world().get::<BlasterShot>(confirmed), Some(&shot));
    assert!(
        app.world()
            .get::<lightyear::prediction::despawn::PredictionDisable>(confirmed)
            .is_none()
    );
}
