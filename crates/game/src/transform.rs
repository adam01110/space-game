use bevy::prelude::*;
use project_protocol::{Player, PlayerHeading, PlayerPosition};

pub(super) fn sync_player_transforms(
    mut players: Query<(&PlayerPosition, &PlayerHeading, &mut Transform), With<Player>>,
) {
    for (position, heading, mut transform) in &mut players {
        transform.translation.x = position.x;
        transform.translation.y = position.y;

        transform.rotation = Quat::from_rotation_z(heading.0);
    }
}
