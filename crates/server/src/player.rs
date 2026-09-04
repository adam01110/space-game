use bevy::prelude::*;
use lightyear::{connection::client::Connected, prelude::server::*, prelude::*};

use project_game::PlayerBundle;

pub(super) fn spawn_player_for_client(
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
