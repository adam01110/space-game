mod restoration;
mod suspension;

use std::time::Duration;

use bevy::{
    ecs::system::RunSystemOnce, prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy,
    window::PrimaryWindow,
};
use lightyear::prelude::{
    ConfirmedHistory, PredictionManager, ReplicationCheckpointMap, Tick, client::ClientPlugins,
};

use project_game::{ClientSimulationPlugin, GamePlugin, SERVER_UPS};
use project_protocol::ProtocolPlugin;

use crate::focus::{ClientFocusPlugin, observe_focus};

// A headless client with prediction, protocol, game and focus plugins installed.
fn client() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        TransformPlugin,
        ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / SERVER_UPS),
        },
        ProtocolPlugin,
        GamePlugin,
        ClientSimulationPlugin,
        ClientFocusPlugin,
    ));
    app.insert_resource(PredictionManager::default());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));
    app.world_mut().spawn((
        Window {
            focused: false,
            ..default()
        },
        PrimaryWindow,
    ));
    app.finish();
    app.cleanup();
    app.update();
    app
}

fn focus(app: &mut App, focused: bool) {
    let world = app.world_mut();
    let mut query = world.query::<&mut Window>();
    query.single_mut(world).expect("window").focused = focused;
    world.run_system_once(observe_focus).expect("observe focus");
}

// Complete a checkpoint the way replication does: record it, then confirm it.
fn checkpoint(app: &mut App, tick: u32) {
    let mut map = app.world_mut().resource_mut::<ReplicationCheckpointMap>();
    let replication_tick = map.last_confirmed_replicon_tick().unwrap_or_default() + 1;
    map.record(replication_tick, Tick(tick));
    assert_eq!(
        map.record_last_confirmed_tick(replication_tick),
        Some(Tick(tick))
    );
}

fn seed<C: Component + Clone + PartialEq>(app: &mut App, entity: Entity, tick: Tick, value: C) {
    let mut history = ConfirmedHistory::<C>::default();
    history.insert_present(tick, value);
    app.world_mut().entity_mut(entity).insert(history);
}
