use bevy::prelude::*;
use lightyear::prelude::{input::native::InputMarker, Controlled, Interpolated, Predicted};

use project_protocol::{Player, PlayerInput};

pub(super) struct ClientPlayerPlugin;

impl Plugin for ClientPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_controlled_player)
            .add_observer(add_predicted_player_visual)
            .add_observer(add_interpolated_player_visual);
    }
}

fn prepare_controlled_player(
    trigger: On<Add, Controlled>,
    players: Query<(), (With<Player>, Without<InputMarker<PlayerInput>>)>,
    mut commands: Commands,
) {
    if players.contains(trigger.entity) {
        commands
            .entity(trigger.entity)
            .insert(InputMarker::<PlayerInput>::default());
    }
}

fn add_predicted_player_visual(trigger: On<Add, (Player, Predicted)>, mut commands: Commands) {
    add_player_visual(&mut commands, trigger.entity, Color::srgb(0.35, 0.75, 1.0));
}

fn add_interpolated_player_visual(
    trigger: On<Add, (Player, Interpolated)>,
    mut commands: Commands,
) {
    add_player_visual(&mut commands, trigger.entity, Color::srgb(1.0, 0.4, 0.35));
}

fn add_player_visual(commands: &mut Commands, entity: Entity, color: Color) {
    commands.entity(entity).insert((
        Sprite::from_color(color, Vec2::new(28.0, 44.0)),
        Transform::default(),
    ));
}
