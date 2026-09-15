use std::net::SocketAddr;

use bevy::prelude::*;
use lightyear::{netcode::NetcodeServer, prelude::server::*, prelude::*};

use project_protocol::{PROTOCOL_ID, SERVER_PORT, security::encode_hex};

use super::{auth::AuthService, security::ServerKey};

pub(super) struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_server, start_server).chain())
            .add_observer(prepare_client_link);
    }
}

fn spawn_server(mut commands: Commands, key: Res<ServerKey>) {
    let address = SocketAddr::from(([0, 0, 0, 0], SERVER_PORT));
    let certificate = Identity::self_signed(vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ])
    .expect("failed to generate a WebTransport certificate");

    let leaf = certificate
        .certificate_chain()
        .as_slice()
        .first()
        .expect("generated identity must contain a certificate");
    let certificate_digest = encode_hex(leaf.hash().as_ref());
    let auth =
        AuthService::start(key.0, certificate_digest).expect("failed to start the guest endpoint");
    commands.insert_resource(auth);

    commands.spawn((
        Name::new("Server"),
        Server::new(None),
        NetcodeServer::new(NetcodeConfig {
            protocol_id: PROTOCOL_ID,
            private_key: key.0,
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
