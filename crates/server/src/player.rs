use avian2d::prelude::Position;
use bevy::prelude::*;
use lightyear::{connection::client::Connected, prelude::server::*, prelude::*};

use project_game::{PLAYER_RADIUS, PlayerBundle};
use project_protocol::CircleBody;

pub(super) struct ServerPlayerPlugin;

impl Plugin for ServerPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_player_for_client);
    }
}

fn spawn_player_for_client(
    trigger: On<Add, Connected>,
    // ClientOf selects connection entities; RemoteId provides the peer ID assigned as owner.
    clients: Query<&RemoteId, With<ClientOf>>,
    bodies: Query<(&Position, &CircleBody)>,
    mut next_spawn_slot: Local<u64>,
    mut commands: Commands,
) {
    let Ok(remote_id) = clients.get(trigger.entity) else {
        return;
    };

    let peer_id = remote_id.0;
    // Reserve distinct slots even if multiple connections arrive before commands flush.
    // Skip slots occupied by an existing body instead of spawning circles on top of it.
    let spawn_position = loop {
        #[expect(
            clippy::cast_precision_loss,
            reason = "World coordinates use f32; the overlap check rejects rounded occupied slots"
        )]
        let candidate = Vec2::new(*next_spawn_slot as f32 * 80.0 - 160.0, 0.0);
        *next_spawn_slot += 1;
        if bodies.iter().all(|(position, circle)| {
            position.0.distance(candidate) > PLAYER_RADIUS + circle.radius + 2.0
        }) {
            break candidate;
        }
    };

    let player = commands
        .spawn((
            Name::new(format!("Player {peer_id:?}")),
            PlayerBundle::new(spawn_position),
            Replicate::to_clients(NetworkTarget::All),
            // Contacts must use the same simulation tick on both sides. Delayed remote
            // interpolation would put the other collider in the past.
            PredictionTarget::to_clients(NetworkTarget::All),
            ControlledBy {
                owner: trigger.entity,
                lifetime: default(),
            },
        ))
        .id();

    info!("spawned {player:?} for {peer_id:?}");
}
