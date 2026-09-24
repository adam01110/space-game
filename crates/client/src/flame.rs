use bevy::prelude::*;
use bevy_resvg::prelude::{Svg, SvgFile};
use lightyear::prelude::input::native::ActionState;

use space_game_protocol::{PlayerBoost, PlayerInput};

const FLAME_SIZE: Vec2 = Vec2::new(14.0, 32.0);
const SPRITE_INTERVAL_MILLIS: u128 = 90;

#[derive(Resource)]
pub(super) struct FlameAssets {
    red: [Handle<SvgFile>; 3],
    blue: [Handle<SvgFile>; 3],
}

impl FromWorld for FlameAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            red: [
                assets.load("sprites/flame-red-0.svg"),
                assets.load("sprites/flame-red-1.svg"),
                assets.load("sprites/flame-red-2.svg"),
            ],
            blue: [
                assets.load("sprites/flame-blue-0.svg"),
                assets.load("sprites/flame-blue-1.svg"),
                assets.load("sprites/flame-blue-2.svg"),
            ],
        }
    }
}

#[derive(Resource, Default)]
struct FlameImages {
    red: [Option<Handle<Image>>; 3],
    blue: [Option<Handle<Image>>; 3],
}

impl FlameImages {
    fn frame(&self, color: Option<FlameColor>, index: usize) -> Option<&Handle<Image>> {
        let frames = match color? {
            FlameColor::Red => &self.red,
            FlameColor::Blue => &self.blue,
        };

        frames.get(index)?.as_ref()
    }
}

#[derive(Component)]
struct PlayerFlame {
    player: Entity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FlameColor {
    Red,
    Blue,
}

pub(super) struct ClientFlamePlugin;

impl Plugin for ClientFlamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlameAssets>()
            .init_resource::<FlameImages>()
            .add_systems(Update, (size_player_flames, animate_player_flames).chain());
    }
}

pub(super) fn add_player_flame(commands: &mut Commands, player: Entity, assets: &FlameAssets) {
    commands.entity(player).with_children(|children| {
        children.spawn((
            PlayerFlame { player },
            Svg(assets.red[0].clone()),
            Transform::from_xyz(0.0, -32.0, -0.1),
            Visibility::Hidden,
        ));
    });
}

fn size_player_flames(mut flames: Query<&mut Sprite, (With<PlayerFlame>, Added<Sprite>)>) {
    for mut sprite in &mut flames {
        sprite.custom_size = Some(FLAME_SIZE);
    }
}

fn cache_flame_sprites(
    sources: &[Handle<SvgFile>; 3],
    targets: &mut [Option<Handle<Image>>; 3],
    svg_files: &Assets<SvgFile>,
    images: &mut Assets<Image>,
) {
    for (source, target) in sources.iter().zip(targets.iter_mut()) {
        if target.is_none() {
            *target = svg_files.get(source).map(|svg| images.add(svg.0.clone()));
        }
    }
}

fn animate_player_flames(
    time: Res<Time>,
    assets: Res<FlameAssets>,
    svg_files: Res<Assets<SvgFile>>,
    mut images: ResMut<Assets<Image>>,
    mut sprite_images: ResMut<FlameImages>,
    players: Query<(&PlayerBoost, &ActionState<PlayerInput>)>,
    mut thrusters: Query<(&PlayerFlame, &mut Sprite, &mut Visibility)>,
) {
    // bevy_resvg loads a static SVG into Sprite.image but does not react when its Svg handle
    // changes. Cache rasterised sprite variants and switch the image handle instead.
    cache_flame_sprites(&assets.red, &mut sprite_images.red, &svg_files, &mut images);
    cache_flame_sprites(
        &assets.blue,
        &mut sprite_images.blue,
        &svg_files,
        &mut images,
    );

    let sprite_index = usize::try_from(time.elapsed().as_millis() / SPRITE_INTERVAL_MILLIS % 3)
        .unwrap_or_default();
    for (flame, mut sprite, mut visibility) in &mut thrusters {
        let color = players
            .get(flame.player)
            .ok()
            .and_then(|(boost, input)| flame_color(&input.0, boost));

        update_flame(
            &mut sprite,
            &mut visibility,
            color,
            &sprite_images,
            sprite_index,
        );
    }
}

fn update_flame(
    sprite: &mut Sprite,
    visibility: &mut Visibility,
    color: Option<FlameColor>,
    images: &FlameImages,
    index: usize,
) {
    *visibility = match color.is_some() {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    };

    if let Some(image) = images.frame(color, index) {
        sprite.image.clone_from(image);
    }
}

pub(super) fn flame_color(input: &PlayerInput, boost: &PlayerBoost) -> Option<FlameColor> {
    match input.boost && boost.0.units() > 0 {
        true => Some(FlameColor::Blue),
        false => (input.movement.dot(input.aim) > 0.0).then_some(FlameColor::Red),
    }
}
