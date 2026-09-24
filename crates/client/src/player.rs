use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use bevy_resvg::prelude::{Svg, SvgColor, SvgFile};
use lightyear::prelude::{Controlled, Predicted, input::native::InputMarker};

use space_game_protocol::{Player, PlayerInput};

use crate::{
    flame::{FlameAssets, add_player_flame},
    palette::Palette,
};

// The ship's rendered rectangle in world units.
const PLAYER_SIZE: Vec2 = Vec2::new(33.6, 52.8);
const PLAYER_SPRITE: &str = "sprites/player.svg";

pub(super) struct ClientPlayerPlugin;

#[derive(Resource)]
pub(super) struct PlayerSprite(pub(super) Handle<SvgFile>);

impl FromWorld for PlayerSprite {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AssetServer>().load(PLAYER_SPRITE))
    }
}

impl Plugin for ClientPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSprite>()
            .add_observer(prepare_controlled_player)
            .add_systems(Update, (add_player_visuals, size_player_sprites).chain());
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
pub(super) fn add_player_visuals(
    players: Query<
        (Entity, Has<Controlled>),
        (
            With<Player>,
            With<Predicted>,
            With<Position>,
            With<Rotation>,
            Without<Svg>,
        ),
    >,
    sprite: Res<PlayerSprite>,
    svg_files: Res<Assets<SvgFile>>,
    flames: Res<FlameAssets>,
    mut commands: Commands,
) {
    if !svg_files.contains(&sprite.0) {
        return;
    }

    for (entity, controlled) in &players {
        let color = match controlled {
            true => Palette::Teal.color(),
            false => Palette::Rose.color(),
        };

        commands.entity(entity).insert((
            Svg(sprite.0.clone()),
            SvgColor(color),
            Visibility::Inherited,
        ));

        add_player_flame(&mut commands, entity, &flames);
    }
}

// The SVG rasterises at its declared size, so the sprite would inherit the SVG's pixel
// dimensions; the ship keeps the rectangle the collider was tuned against.
fn size_player_sprites(mut sprites: Query<&mut Sprite, (With<Player>, Added<Sprite>)>) {
    for mut sprite in &mut sprites {
        sprite.custom_size = Some(PLAYER_SIZE);
    }
}
