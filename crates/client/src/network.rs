mod connection;
pub(crate) mod policy;
pub(crate) mod recovery;

use std::time::Duration;

use bevy::prelude::*;
use crossbeam_channel::Receiver;
use lightyear::prelude::client::*;

use super::{guest, plugins::ClientStartup};
use crate::palette::Palette;

use connection::{monitor_client, poll_guest, retry_connection};

pub(super) struct ClientNetworkPlugin;

const RETRY_INTERVAL: Duration = Duration::from_secs(4);

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        recovery::install(app);
        app.insert_resource(policy::timeline_config())
            .insert_resource(policy::prediction_manager())
            .add_observer(connection::log_disconnect)
            .add_systems(Startup, setup_connection.in_set(ClientStartup::Connection))
            .add_systems(Update, update_connection);
    }
}

// Guest credential retrieval and the client link it produces, with the retry state machine.
#[derive(Resource, Default)]
pub(super) struct GuestConnection {
    pub(super) pending: Option<Receiver<guest::GuestResult>>,
    pub(super) client: Option<Entity>,
    pub(super) started: Duration,
    pub(super) message: String,
    pub(super) can_retry: bool,
}

impl GuestConnection {
    pub(super) fn fail(&mut self, message: &str, now: Duration) {
        self.pending = None;
        self.started = now;
        self.message = format!("{message}. Retrying in 4 seconds. Press R to retry now.");
        self.can_retry = true;
    }

    pub(super) fn request(&mut self, now: Duration) {
        self.started = now;
        self.can_retry = false;

        match guest::request() {
            Ok(receiver) => {
                self.pending = Some(receiver);
                "Requesting guest access...".clone_into(&mut self.message);
            }
            Err(message) => self.fail(&message, now),
        }
    }
}

#[derive(Component)]
struct ConnectionStatus;

fn setup_connection(mut commands: Commands, time: Res<Time<Real>>) {
    let mut connection = GuestConnection::default();
    connection.request(time.elapsed());

    commands.spawn((
        ConnectionStatus,
        Text::new(connection.message.clone()),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Palette::Cream.color()),
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            ..default()
        },
    ));

    commands.insert_resource(connection);
}

fn update_status(status: &mut Query<&mut Text, With<ConnectionStatus>>, message: &str) {
    for mut text in status {
        if text.0 != message {
            message.clone_into(&mut text.0);
        }
    }
}

fn update_connection(
    mut commands: Commands,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut connection: ResMut<GuestConnection>,
    clients: Query<(Has<Connected>, Option<&Disconnected>), With<Client>>,
    suspension: Res<recovery::Suspension>,
    mut status: Query<&mut Text, With<ConnectionStatus>>,
) {
    let now = time.elapsed();
    if suspension.is_suspended() {
        update_status(&mut status, &connection.message);
        return;
    }

    if connection.can_retry
        && (keys.just_pressed(KeyCode::KeyR)
            || now.saturating_sub(connection.started) >= RETRY_INTERVAL)
    {
        retry_connection(&mut commands, &mut connection, now);
    }

    match connection.pending.is_some() {
        true => poll_guest(&mut commands, &mut connection, now),
        false => match connection.can_retry {
            true => {}
            false => monitor_client(&mut commands, &mut connection, &clients, now),
        },
    }

    update_status(&mut status, &connection.message);
}
