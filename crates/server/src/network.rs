use std::net::SocketAddr;

use bevy::prelude::*;
use lightyear::{netcode::NetcodeServer, prelude::server::*, prelude::*};

use project_protocol::{PRIVATE_KEY, PROTOCOL_ID, SERVER_PORT};

pub(super) fn spawn_server(mut commands: Commands) {
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

pub(super) fn start_server(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}

pub(super) fn prepare_client_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((Name::new("Client connection"), ReplicationSender));
}
