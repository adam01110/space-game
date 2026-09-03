use std::net::{Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use lightyear::{
    netcode::{client_plugin::NetcodeConfig, generate_key, NetcodeClient},
    prelude::{client::*, *},
};
use project_protocol::{PRIVATE_KEY, PROTOCOL_ID, SERVER_PORT};

pub(super) fn spawn_client(mut commands: Commands) {
    let client_id = u64::from_le_bytes(
        generate_key()[..size_of::<u64>()]
            .try_into()
            .expect("client ID source must contain eight bytes"),
    );
    let server_address = SocketAddr::from((Ipv4Addr::LOCALHOST, SERVER_PORT));
    let authentication = Authentication::Manual {
        server_addr: server_address,
        client_id,
        private_key: PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };

    commands.insert_resource(PredictionManager::default());
    commands.spawn((
        Name::new(format!("Client {client_id}")),
        Client,
        ReplicationReceiver,
        Link::default(),
        LocalAddr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))),
        PeerAddr(server_address),
        PingManager::default(),
        NetcodeClient::new(authentication, NetcodeConfig::default())
            .expect("failed to configure Netcode client"),
        WebTransportClientIo {
            certificate_digest: String::new(),
            target: None,
        },
    ));
}

pub(super) fn connect_client(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}
