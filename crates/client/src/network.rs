use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use bevy::prelude::*;
use crossbeam_channel::{Receiver, TryRecvError};
use lightyear::{
    netcode::{client_plugin::NetcodeConfig, NetcodeClient},
    prelude::{client::*, *},
};

use crate::guest::{self, Credentials, GuestResult};

#[derive(Resource, Default)]
pub(super) struct GuestConnection {
    pending: Option<Receiver<GuestResult>>,
    client: Option<Entity>,
    started: Duration,
    message: String,
    can_retry: bool,
}

impl GuestConnection {
    fn fail(&mut self, message: &str) {
        self.pending = None;
        self.message = format!("{message}. Press R to retry.");
        self.can_retry = true;
    }

    fn request(&mut self, now: Duration) {
        self.started = now;
        self.can_retry = false;
        match guest::request() {
            Ok(receiver) => {
                self.pending = Some(receiver);
                "Requesting guest access...".clone_into(&mut self.message);
            }
            Err(message) => self.fail(&message),
        }
    }
}

#[derive(Component)]
pub(super) struct ConnectionStatus;

pub(super) fn setup_connection(mut commands: Commands, time: Res<Time<Real>>) {
    let mut connection = GuestConnection::default();
    connection.request(time.elapsed());
    commands.spawn((
        ConnectionStatus,
        Text::new(connection.message.clone()),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            ..default()
        },
    ));
    commands.insert_resource(connection);
    commands.insert_resource(PredictionManager::default());
}

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

fn retry_connection(commands: &mut Commands, connection: &mut GuestConnection, now: Duration) {
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
        Err(message) => connection.fail(&message),
    }
}

fn handle_empty_guest_response(connection: &mut GuestConnection, now: Duration) {
    if now.saturating_sub(connection.started) > Duration::from_secs(15) {
        connection.fail("Guest request timed out");
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
        Ok(Err(message)) => connection.fail(&message),
        Err(TryRecvError::Disconnected) => connection.fail("Guest request ended unexpectedly"),
        Err(TryRecvError::Empty) => handle_empty_guest_response(connection, now),
    }
}

fn poll_guest(commands: &mut Commands, connection: &mut GuestConnection, now: Duration) {
    let Some(receiver) = &connection.pending else {
        return;
    };
    handle_guest_result(commands, connection, receiver.try_recv(), now);
}

fn monitor_client(
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
        Ok((false, true)) => connection.fail("Disconnected from the server"),
        _ if now.saturating_sub(connection.started) > Duration::from_secs(15) => {
            commands.trigger(Disconnect { entity });
            connection.fail("Secure connection timed out");
        }
        _ => {}
    }
}

fn update_status(status: &mut Query<&mut Text, With<ConnectionStatus>>, message: &str) {
    for mut text in status {
        if text.0 != message {
            message.clone_into(&mut text.0);
        }
    }
}

pub(super) fn update_connection(
    mut commands: Commands,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut connection: ResMut<GuestConnection>,
    clients: Query<(Has<Connected>, Has<Disconnected>), With<Client>>,
    mut status: Query<&mut Text, With<ConnectionStatus>>,
) {
    let now = time.elapsed();
    if connection.can_retry && keys.just_pressed(KeyCode::KeyR) {
        retry_connection(&mut commands, &mut connection, now);
    }
    if connection.pending.is_some() {
        poll_guest(&mut commands, &mut connection, now);
    } else if !connection.can_retry {
        monitor_client(&mut commands, &mut connection, &clients, now);
    }
    update_status(&mut status, &connection.message);
}
