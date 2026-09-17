use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use bevy::prelude::*;
use crossbeam_channel::TryRecvError;
use lightyear::{
    netcode::{NetcodeClient, client_plugin::NetcodeConfig},
    prelude::{client::*, *},
};

use super::GuestConnection;
use crate::guest::{Credentials, GuestResult};

fn spawn_client(commands: &mut Commands, credentials: Credentials) -> Result<Entity, String> {
    let netcode = NetcodeClient::new(
        Authentication::Token(credentials.token),
        NetcodeConfig::default(),
    )
    .map_err(|error| format!("Invalid connection token: {error}"))?;
    let entity = commands
        .spawn((
            Name::new("Client"),
            Client,
            ReplicationReceiver,
            Link::default(),
            LocalAddr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))),
            PingManager::default(),
            netcode,
            WebTransportClientIo {
                certificate_digest: credentials.certificate_digest,
                target: None,
            },
        ))
        .id();
    commands.trigger(Connect { entity });
    Ok(entity)
}

pub(super) fn retry_connection(
    commands: &mut Commands,
    connection: &mut GuestConnection,
    now: Duration,
) {
    if let Some(entity) = connection.client.take() {
        commands.trigger(Disconnect { entity });
        commands.entity(entity).despawn();
    }
    connection.request(now);
}

fn accept_credentials(
    commands: &mut Commands,
    connection: &mut GuestConnection,
    credentials: Credentials,
    now: Duration,
) {
    connection.pending = None;
    match spawn_client(commands, credentials) {
        Ok(entity) => {
            connection.client = Some(entity);
            connection.started = now;
            "Connecting securely...".clone_into(&mut connection.message);
        }
        Err(message) => connection.fail(&message, now),
    }
}

fn handle_empty_guest_response(connection: &mut GuestConnection, now: Duration) {
    if now.saturating_sub(connection.started) > Duration::from_secs(15) {
        connection.fail("Guest request timed out", now);
    }
}

fn handle_guest_result(
    commands: &mut Commands,
    connection: &mut GuestConnection,
    result: Result<GuestResult, TryRecvError>,
    now: Duration,
) {
    match result {
        Ok(Ok(credentials)) => accept_credentials(commands, connection, credentials, now),
        Ok(Err(message)) => connection.fail(&message, now),
        Err(TryRecvError::Disconnected) => connection.fail("Guest request ended unexpectedly", now),
        Err(TryRecvError::Empty) => handle_empty_guest_response(connection, now),
    }
}

pub(super) fn poll_guest(commands: &mut Commands, connection: &mut GuestConnection, now: Duration) {
    let Some(receiver) = &connection.pending else {
        return;
    };

    handle_guest_result(commands, connection, receiver.try_recv(), now);
}

pub(super) fn monitor_client(
    commands: &mut Commands,
    connection: &mut GuestConnection,
    clients: &Query<(Has<Connected>, Has<Disconnected>), With<Client>>,
    now: Duration,
) {
    let Some(entity) = connection.client else {
        return;
    };

    match clients.get(entity) {
        Ok((true, _)) => connection.message.clear(),
        Ok((false, true)) => connection.fail("Disconnected from the server", now),
        _ if now.saturating_sub(connection.started) > Duration::from_secs(15) => {
            commands.trigger(Disconnect { entity });
            connection.fail("Secure connection timed out", now);
        }
        _ => {}
    }
}
