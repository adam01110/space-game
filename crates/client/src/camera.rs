use bevy::prelude::*;

pub(super) fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
