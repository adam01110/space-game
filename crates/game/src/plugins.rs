use avian2d::prelude::PhysicsSystems;
use bevy::prelude::*;
use lightyear::prelude::PredictionSystems;

use crate::{
    abilities::{ClientAbilitiesPlugin, ServerAbilitiesPlugin},
    arena::{
        advance_predicted_arena, contain_authoritative_bodies, contain_predicted_bodies,
        install_arena_interpolation, prepare_predicted_arena, resize_arena,
    },
    movement::{move_authoritative_players, move_predicted_players},
    physics::{install_physics, prepare_authoritative_body, prepare_predicted_body},
};

pub const SERVER_UPS: f64 = 60.0;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(SERVER_UPS));
        install_physics(app);
        install_arena_interpolation(app);
    }
}

pub struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_predicted_arena)
            .add_systems(FixedUpdate, advance_predicted_arena)
            .add_observer(prepare_predicted_body)
            .add_systems(
                FixedPostUpdate,
                contain_predicted_bodies
                    .after(PhysicsSystems::StepSimulation)
                    .before(PhysicsSystems::Writeback)
                    .before(PredictionSystems::UpdateHistory),
            );
        /*
        Input is buffered in FixedPreUpdate. Apply desired velocity here, then Avian
        resolves contacts in FixedPostUpdate before Lightyear records prediction history.
        */
        app.add_systems(FixedUpdate, move_predicted_players)
            .add_plugins(ClientAbilitiesPlugin);
    }
}

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_authoritative_body)
            .add_systems(FixedUpdate, resize_arena)
            .add_systems(
                FixedPostUpdate,
                contain_authoritative_bodies
                    .after(PhysicsSystems::StepSimulation)
                    .before(PhysicsSystems::Writeback)
                    .before(PredictionSystems::UpdateHistory),
            );
        app.add_systems(FixedUpdate, move_authoritative_players)
            .add_plugins(ServerAbilitiesPlugin);
    }
}
