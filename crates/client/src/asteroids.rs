use avian2d::prelude::Position;
use bevy::prelude::*;
use bevy_resvg::prelude::{Svg, SvgFile};
use lightyear::prelude::Predicted;

use space_game_protocol::Asteroid;

const SPRITES: [&str; 3] = [
    "sprites/asteroid-0.svg",
    "sprites/asteroid-1.svg",
    "sprites/asteroid-2.svg",
];

#[derive(Resource)]
struct AsteroidSprites([Handle<SvgFile>; 3]);

impl FromWorld for AsteroidSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        Self(SPRITES.map(|path| server.load(path)))
    }
}

#[derive(Component)]
struct AsteroidVisual;

pub(super) struct ClientAsteroidPlugin;

impl Plugin for ClientAsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AsteroidSprites>()
            .add_systems(Update, (add_visuals, size_visuals).chain());
    }
}

type UnrenderedAsteroids<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Asteroid),
    (With<Predicted>, With<Position>, Without<AsteroidVisual>),
>;

type NewVisuals<'w, 's> =
    Query<'w, 's, (&'static ChildOf, &'static mut Sprite), (With<AsteroidVisual>, Added<Sprite>)>;

fn add_visuals(
    asteroids: UnrenderedAsteroids,
    sprites: Res<AsteroidSprites>,
    assets: Res<Assets<SvgFile>>,
    mut commands: Commands,
) {
    for (entity, asteroid) in &asteroids {
        let Some(handle) = sprites.0.get(usize::from(asteroid.variant)) else {
            continue;
        };
        if !assets.contains(handle) {
            continue;
        }
        commands
            .entity(entity)
            .insert(AsteroidVisual)
            .with_children(|children| {
                children.spawn((
                    Svg(handle.clone()),
                    AsteroidVisual,
                    Transform::from_rotation(Quat::from_rotation_z(asteroid.angle))
                        .with_scale(asteroid.stretch.extend(1.0)),
                ));
            });
    }
}

fn size_visuals(mut visuals: NewVisuals, asteroids: Query<&Asteroid>) {
    for (parent, mut sprite) in &mut visuals {
        if let Ok(asteroid) = asteroids.get(parent.parent()) {
            sprite.custom_size = Some(Vec2::splat(asteroid.radius * 2.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_resvg::resvg::usvg::{Options, Tree};

    #[test]
    fn all_asteroid_sprites_parse() {
        for source in [
            include_str!("../../../assets/sprites/asteroid-0.svg"),
            include_str!("../../../assets/sprites/asteroid-1.svg"),
            include_str!("../../../assets/sprites/asteroid-2.svg"),
        ] {
            Tree::from_data(source.as_bytes(), &Options::default()).expect("valid asteroid sprite");
        }
    }
}
