use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::{color::Alpha, prelude::*, transform::TransformSystems};
use lightyear::frame_interpolation::FrameInterpolationSystems;
use lightyear::prelude::Predicted;

use space_game_protocol::ArenaBoundary;

use crate::palette::Palette;

// Number of pylons spaced evenly around the rim. A fixed count keeps the pylon entities stable
// while the radius animates, so nothing is spawned or despawned mid-game.
pub(super) const BEACON_COUNT: usize = 16;
// Mast rectangle in world units. Its long axis is placed along the radius, with the mast outside
// the boundary, so it never overlaps a contained body.
pub(super) const BEACON_MAST: Vec2 = Vec2::new(6.0, 24.0);
// Side of the square light on the inner tip of the mast.
pub(super) const BEACON_LIGHT: f32 = 8.0;
// Blinks per second.
const BEACON_BLINK_HZ: f32 = 0.6;
// Brightness the light keeps at the dimmest point of the pulse, so the rim stays marked.
pub(super) const BEACON_DIM: f32 = 0.15;
// Behind the ships and shots, in front of the streamed backdrop.
const BEACON_Z: f32 = -1.0;
const MAST_COLOR: Color = Palette::Indigo.color();
const LIGHT_COLOR: Color = Palette::Citron.color();

pub(super) struct ClientBeaconPlugin;

impl Plugin for ClientBeaconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_beacons).add_systems(
            PostUpdate,
            (follow_arena, pulse_lights)
                // The arena radius is interpolated for rendering, so the pylons read the same
                // smoothed value the rim is drawn at instead of the raw predicted one.
                .after(FrameInterpolationSystems::Interpolate)
                .before(TransformSystems::Propagate),
        );
    }
}

// The mast of one pylon, positioned on the rim every frame.
#[derive(Component)]
pub(super) struct BeaconPylon(pub(super) usize);

// The blinking light at the inner tip of a mast.
#[derive(Component)]
pub(super) struct BeaconLight {
    // Position within the blink cycle, so neighbouring lights pulse out of step.
    pub(super) phase: f32,
}

type PylonQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static BeaconPylon,
        &'static mut Transform,
        &'static mut Visibility,
    ),
>;

// Angle of pylon `index` around the rim, measured from +X.
#[expect(
    clippy::cast_precision_loss,
    reason = "The pylon count is a small fixed constant"
)]
fn beacon_angle(index: usize) -> f32 {
    TAU * index as f32 / BEACON_COUNT as f32
}

// Spawned once, hidden, and placed by `follow_arena` as soon as a predicted arena exists. A guest
// that has not connected yet has no arena, so the pylons stay out of the menu.
fn spawn_beacons(mut commands: Commands) {
    for index in 0..BEACON_COUNT {
        let mast = commands
            .spawn((
                Sprite::from_color(MAST_COLOR, BEACON_MAST),
                Transform::from_xyz(0.0, 0.0, BEACON_Z),
                Visibility::Hidden,
                BeaconPylon(index),
            ))
            .id();

        // The light inherits the mast's transform and visibility, so it follows the rim and
        // disappears with the mast without a second placement pass.
        commands.spawn((
            Sprite::from_color(LIGHT_COLOR, Vec2::splat(BEACON_LIGHT)),
            Transform::from_xyz(0.0, -BEACON_MAST.y / 2.0, 0.0),
            BeaconLight {
                phase: beacon_angle(index) / TAU,
            },
            ChildOf(mast),
        ));
    }
}

// Places every mast on the boundary. The mast centre sits half a mast outside the radius and the
// light sits on the inner tip, so the lights trace the containment circle exactly while it grows
// or shrinks.
fn follow_arena(arenas: Query<&ArenaBoundary, With<Predicted>>, mut pylons: PylonQuery) {
    let radius = arenas.single().map(|arena| arena.radius).ok();

    for (pylon, mut transform, mut visibility) in &mut pylons {
        let Some(radius) = radius else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let angle = beacon_angle(pylon.0);
        let outward = Vec2::new(angle.cos(), angle.sin());

        *transform = Transform::from_translation(
            (outward * (radius + BEACON_MAST.y / 2.0)).extend(BEACON_Z),
        )
        // The sprite's long axis is its local +Y, which this turns to point along the radius.
        .with_rotation(Quat::from_rotation_z(angle - FRAC_PI_2));
        *visibility = Visibility::Inherited;
    }
}

// Dims and brightens each light on its own phase, which reads as a signal chasing around the rim
// rather than the whole ring blinking in unison.
fn pulse_lights(time: Res<Time>, mut lights: Query<(&BeaconLight, &mut Sprite)>) {
    let elapsed = time.elapsed_secs();

    for (light, mut sprite) in &mut lights {
        let wave = ((elapsed * BEACON_BLINK_HZ + light.phase) * TAU).sin() * 0.5 + 0.5;
        sprite.color = LIGHT_COLOR.with_alpha(BEACON_DIM + (1.0 - BEACON_DIM) * wave);
    }
}
