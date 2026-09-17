use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{Controlled, Predicted, input::native::InputMarker};

use space_game_protocol::{Player, PlayerInput};

pub(super) struct ClientPlayerPlugin;

impl Plugin for ClientPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare_controlled_player)
            .add_systems(Update, add_player_visuals);
    }
}

type NeedsInputMarker = (
    With<Player>,
    With<Controlled>,
    Without<InputMarker<PlayerInput>>,
);

fn prepare_controlled_player(
    trigger: On<Add, (Player, Controlled)>,
    players: Query<(), NeedsInputMarker>,
    mut commands: Commands,
) {
    if players.contains(trigger.entity) {
        commands
            .entity(trigger.entity)
            .insert(InputMarker::<PlayerInput>::default());
    }
}

// Wait for both pose components and ownership metadata; never overwrite a physics Transform.
#[expect(
    clippy::type_complexity,
    reason = "The query waits for the complete predicted physics pose before adding visuals"
)]
fn add_player_visuals(
    players: Query<
        (Entity, Has<Controlled>),
        (
            With<Player>,
            With<Predicted>,
            With<Position>,
            With<Rotation>,
            Without<Sprite>,
        ),
    >,
    mut commands: Commands,
) {
    for (entity, controlled) in &players {
        let color = match controlled {
            true => Color::srgb(0.35, 0.75, 1.0),
            false => Color::srgb(1.0, 0.4, 0.35),
        };

        commands
            .entity(entity)
            .insert(Sprite::from_color(color, Vec2::new(33.6, 52.8)));
    }
}
