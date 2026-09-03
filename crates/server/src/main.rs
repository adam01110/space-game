use bevy::{prelude::*, state::app::StatesPlugin};
use lightyear::{
    connection::client::Connected,
    netcode::NetcodeServer,
    prelude::{server::*, *},
};
use project_game::{GamePlugin, PlayerBundle, ServerSimulationPlugin};
use project_protocol::{PRIVATE_KEY, PROTOCOL_ID, ProtocolPlugin, SERVER_PORT};
use std::{net::SocketAddr, time::Duration};

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / 60.0),
            )),
            StatesPlugin,
            ServerPlugins::default(),
            ProtocolPlugin,
            GamePlugin,
            ServerSimulationPlugin,
        ))
        .insert_resource(ReplicationMetadata::new(Duration::from_millis(50)))
        .add_systems(Startup, (spawn_server, start_server).chain())
        .add_observer(prepare_client_link)
        .add_observer(spawn_player_for_client)
        .run();
}

fn spawn_server(mut commands: Commands) {
    let address = SocketAddr::from(([0, 0, 0, 0], SERVER_PORT));
    let certificate = Identity::self_signed(vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ])
    .expect("failed to generate a WebTransport certificate");

    commands.spawn((
        Name::new("Server"),
        Server::new(None),
        NetcodeServer::new(NetcodeConfig {
            protocol_id: PROTOCOL_ID,
            private_key: PRIVATE_KEY,
            ..default()
        }),
        LocalAddr(address),
        WebTransportServerIo { certificate },
    ));

    info!("WebTransport server listening on {address}");
}

fn start_server(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}

fn prepare_client_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((Name::new("Client connection"), ReplicationSender));
}

fn spawn_player_for_client(
    trigger: On<Add, Connected>,
    clients: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = clients.get(trigger.entity) else {
        return;
    };
    let peer_id = remote_id.0;
    let spawn_x = (peer_id.to_bits() % 5) as f32 * 80.0 - 160.0;

    let player = commands
        .spawn((
            Name::new(format!("Player {peer_id:?}")),
            PlayerBundle::new(Vec2::new(spawn_x, 0.0)),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(peer_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(peer_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: default(),
            },
        ))
        .id();

    info!("spawned {player:?} for {peer_id:?}");
}
