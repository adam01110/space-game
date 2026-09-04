use bevy::prelude::*;
use lightyear::prelude::input::native::InputMarker;

use project_protocol::{PlayerInput, PlayerPosition};

pub(super) fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub(super) fn follow_player(
    players: Query<&PlayerPosition, With<InputMarker<PlayerInput>>>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    let (Ok(position), Ok(mut camera_transform)) = (players.single(), cameras.single_mut()) else {
        return;
    };

    camera_transform.translation.x = position.x;
    camera_transform.translation.y = position.y;
}
