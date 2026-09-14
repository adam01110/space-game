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
                self.message = "Requesting guest access...".to_owned();
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
            font_size: 20.0,
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
    .map_err(|_error| "Invalid connection token".to_owned())?;
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

pub(super) fn update_connection(
    mut commands: Commands,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut connection: ResMut<GuestConnection>,
    clients: Query<(Has<Connected>, Has<Disconnected>), With<Client>>,
    mut status: Query<&mut Text, With<ConnectionStatus>>,
) {
    if connection.can_retry && keys.just_pressed(KeyCode::KeyR) {
        if let Some(entity) = connection.client.take() {
            commands.trigger(Disconnect { entity });
            commands.entity(entity).despawn();
        }
        connection.request(time.elapsed());
    }
    if let Some(receiver) = &connection.pending {
        match receiver.try_recv() {
            Ok(Ok(credentials)) => {
                connection.pending = None;
                match spawn_client(&mut commands, credentials) {
                    Ok(entity) => {
                        connection.client = Some(entity);
                        connection.started = time.elapsed();
                        connection.message = "Connecting securely...".to_owned();
                    }
                    Err(message) => connection.fail(&message),
                }
            }
            Ok(Err(message)) => connection.fail(&message),
            Err(TryRecvError::Disconnected) => connection.fail("Guest request ended unexpectedly"),
            Err(TryRecvError::Empty) => {
                if time.elapsed().saturating_sub(connection.started) > Duration::from_secs(15) {
                    connection.fail("Guest request timed out");
                }
            }
        }
    } else if !connection.can_retry {
        if let Some(entity) = connection.client {
            match clients.get(entity) {
                Ok((true, _)) => connection.message.clear(),
                Ok((false, true)) => connection.fail("Disconnected from the server"),
                _ => {
                    if time.elapsed().saturating_sub(connection.started) > Duration::from_secs(15) {
                        commands.trigger(Disconnect { entity });
                        connection.fail("Secure connection timed out");
                    }
                }
            }
        }
    }
    for mut text in &mut status {
        if text.0 != connection.message {
            text.0.clone_from(&connection.message);
        }
    }
}
