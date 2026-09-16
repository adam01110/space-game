use std::time::Duration;

use bevy::{
    app::{TaskPoolOptions, TaskPoolPlugin, TaskPoolThreadAssignmentPolicy},
    prelude::*,
    state::app::StatesPlugin,
};
use lightyear::prelude::{ReplicationMetadata, server::*};

use project_game::{GamePlugin, SERVER_UPS, ServerSimulationPlugin};
use project_protocol::ProtocolPlugin;
use project_server::plugins;

/*
A headless simulation produces little parallel work; small fixed pools avoid a
dozen idle threads and their allocator arenas.
*/
fn task_pool_options() -> TaskPoolOptions {
    const POLICY: fn(usize) -> TaskPoolThreadAssignmentPolicy =
        |max_threads| TaskPoolThreadAssignmentPolicy {
            min_threads: 1,
            max_threads,
            percent: 0.0,
            on_thread_spawn: None,
            on_thread_destroy: None,
        };

    TaskPoolOptions {
        min_total_threads: 1,
        max_total_threads: 8,
        io: POLICY(2),
        async_compute: POLICY(2),
        compute: POLICY(4),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(server) = plugins::configure()? else {
        return Ok(());
    };

    App::new()
        .add_plugins((
            MinimalPlugins
                .set(bevy::app::ScheduleRunnerPlugin::run_loop(
                    Duration::from_secs_f64(1.0 / SERVER_UPS),
                ))
                .set(TaskPoolPlugin {
                    task_pool_options: task_pool_options(),
                }),
            bevy::log::LogPlugin {
                // RUST_LOG can override the default level/filter.
                filter: "info,wgpu=error,naga=warn".into(),
                ..default()
            },
            StatesPlugin,
            bevy::transform::TransformPlugin,
            ServerPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
            },
            ProtocolPlugin,
            GamePlugin,
            ServerSimulationPlugin,
            server,
        ))
        .insert_resource(ReplicationMetadata::new(Duration::from_secs_f64(
            1.0 / SERVER_UPS,
        )))
        .run();
    Ok(())
}
