use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use lightyear::prelude::{ReplicationMetadata, server::ServerPlugins};

use project_protocol::{ArenaBoundary, ProtocolPlugin};

use crate::{GamePlugin, SERVER_UPS, ServerSimulationPlugin, arena_radius};

// A headless authoritative server, advanced one fixed tick per `App::update`.
pub(super) fn simulation() -> App {
    build(|_| {})
}

// The same server with the arena spawned at its solo-player radius.
pub(super) fn arena_simulation() -> App {
    build(|app| {
        app.world_mut().spawn(ArenaBoundary::new(arena_radius(0)));
    })
}

fn build(setup: impl FnOnce(&mut App)) -> App {
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
    setup(&mut app);
    app.init_resource::<lightyear::connection::client::PeerMetadata>();
    app.finish();
    app.cleanup();
    app.update();
    app
}
