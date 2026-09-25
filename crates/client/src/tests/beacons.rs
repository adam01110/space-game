use std::time::Duration;

use bevy::{color::Alpha, prelude::*, time::TimeUpdateStrategy};
use lightyear::prelude::Predicted;

use space_game_protocol::ArenaBoundary;

use crate::beacons::{
    BEACON_COUNT, BEACON_DIM, BEACON_LIGHT, BEACON_MAST, BeaconLight, BeaconPylon,
    ClientBeaconPlugin,
};

const FRAME: Duration = Duration::from_millis(16);
const RADIUS: f32 = 1200.0;
// Every placement this test pins is built from the same small constants, so the tolerance only
// keeps the equality off the float lints.
const TOLERANCE: f32 = 1e-3;

// Runs the beacon plugin without a render app: placement and visibility are plain data.
fn beacon_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ClientBeaconPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));

    app
}

fn spawn_arena(app: &mut App, radius: f32) {
    app.world_mut()
        .spawn((ArenaBoundary::new(radius), Predicted));
}

fn set_radius(app: &mut App, radius: f32) {
    let mut query = app
        .world_mut()
        .query_filtered::<&mut ArenaBoundary, With<Predicted>>();
    let mut arena = query.single_mut(app.world_mut()).expect("arena");
    arena.radius = radius;
}

// Mast entity, index and pose, in pylon index order.
fn mast_poses(app: &mut App) -> Vec<(Entity, Vec3, Quat)> {
    let mut query = app
        .world_mut()
        .query::<(Entity, &BeaconPylon, &Transform)>();
    let mut masts = query
        .iter(app.world())
        .map(|(entity, pylon, transform)| {
            (pylon.0, entity, transform.translation, transform.rotation)
        })
        .collect::<Vec<_>>();
    masts.sort_by_key(|(index, _, _, _)| *index);

    masts
        .into_iter()
        .map(|(_, entity, translation, rotation)| (entity, translation, rotation))
        .collect()
}

// Light sprite, placed by the mast's own pose the way the hierarchy does it.
fn lights(app: &mut App) -> Vec<(Vec3, f32)> {
    let masts = mast_poses(app);

    let mut query = app
        .world_mut()
        .query::<(&BeaconLight, &ChildOf, &Transform, &Sprite)>();
    let mut lights = query
        .iter(app.world())
        .map(|(light, parent, transform, sprite)| {
            let (_, translation, rotation) = masts
                .iter()
                .find(|(entity, _, _)| *entity == parent.0)
                .expect("light parent");
            (
                light.phase,
                *translation + *rotation * transform.translation,
                sprite.color.alpha(),
            )
        })
        .collect::<Vec<_>>();
    lights.sort_by(|left, right| left.0.total_cmp(&right.0));

    lights
        .into_iter()
        .map(|(_, placed, alpha)| (placed, alpha))
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "The pylon count is a small fixed constant"
)]
fn outward(index: usize) -> Vec2 {
    let angle = std::f32::consts::TAU * index as f32 / BEACON_COUNT as f32;
    Vec2::new(angle.cos(), angle.sin())
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "expected {expected}, got {actual}"
    );
}

// The mast centre is half a mast outside the boundary and its long axis points along the radius,
// which puts the light exactly on the containment circle.
#[test]
fn pylons_trace_the_arena_rim() {
    let mut app = beacon_app();
    app.update();
    assert_eq!(mast_poses(&mut app).len(), BEACON_COUNT);

    spawn_arena(&mut app, RADIUS);
    app.update();

    for (index, (_, translation, rotation)) in mast_poses(&mut app).into_iter().enumerate() {
        let direction = outward(index);
        assert_close(
            translation.truncate().length(),
            RADIUS + BEACON_MAST.y / 2.0,
        );
        assert_close(translation.truncate().normalize().dot(direction), 1.0);
        assert_close(rotation.mul_vec3(Vec3::Y).truncate().dot(direction), 1.0);
    }

    for (index, (placed, _)) in lights(&mut app).into_iter().enumerate() {
        assert_close(placed.truncate().distance(outward(index) * RADIUS), 0.0);
    }
}

#[test]
fn pylons_stay_hidden_until_a_predicted_arena_exists() {
    let mut app = beacon_app();
    app.update();

    let mut query = app
        .world_mut()
        .query_filtered::<&Visibility, With<BeaconPylon>>();
    let hidden = query
        .iter(app.world())
        .all(|visibility| *visibility == Visibility::Hidden);
    assert!(hidden, "pylons are visible without an arena");

    spawn_arena(&mut app, RADIUS);
    app.update();

    let mut query = app
        .world_mut()
        .query_filtered::<&Visibility, With<BeaconPylon>>();
    let shown = query
        .iter(app.world())
        .all(|visibility| *visibility == Visibility::Inherited);
    assert!(shown, "pylons stayed hidden with an arena present");
}

// The lights are the only cue for where the boundary is while it animates, so they have to move
// with the interpolated radius rather than lag it.
#[test]
fn pylons_follow_a_resized_arena() {
    let mut app = beacon_app();
    spawn_arena(&mut app, RADIUS);
    app.update();

    set_radius(&mut app, RADIUS / 2.0);
    app.update();

    for (_, translation, _) in mast_poses(&mut app) {
        assert_close(
            translation.truncate().length(),
            RADIUS / 2.0 + BEACON_MAST.y / 2.0,
        );
    }

    for (index, (placed, _)) in lights(&mut app).into_iter().enumerate() {
        assert_close(
            placed.truncate().distance(outward(index) * RADIUS / 2.0),
            0.0,
        );
    }
}

// Neighbouring lights are offset in the cycle, so the ring reads as a chasing signal, and the
// pulse never drops far enough for the rim to lose its marker.
#[test]
fn lights_pulse_out_of_phase() {
    let mut app = beacon_app();
    spawn_arena(&mut app, RADIUS);
    app.update();

    let first = lights(&mut app)
        .into_iter()
        .map(|(_, alpha)| alpha)
        .collect::<Vec<_>>();
    assert!(first.iter().all(|alpha| (BEACON_DIM..=1.0).contains(alpha)));

    let (min, max) = first
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), alpha| {
            (min.min(*alpha), max.max(*alpha))
        });
    assert!(
        max - min > TOLERANCE,
        "every light started on the same phase: {first:?}"
    );

    // A quarter of the cycle on, which is far enough for the pulse to have moved.
    for _ in 0..16 {
        app.update();
    }

    let second = lights(&mut app)
        .into_iter()
        .map(|(_, alpha)| alpha)
        .collect::<Vec<_>>();
    assert!(
        second
            .iter()
            .all(|alpha| (BEACON_DIM..=1.0).contains(alpha))
    );
    assert!(
        first
            .iter()
            .zip(&second)
            .any(|(before, after)| (before - after).abs() > TOLERANCE),
        "the pulse did not advance: {first:?} then {second:?}"
    );
}

// The light is a child of the mast, so it has to sit on the mast's inner tip and be its own size
// rather than inherit the mast rectangle.
#[test]
fn lights_are_a_square_on_the_inner_tip() {
    let mut app = beacon_app();
    app.update();

    let mut query = app
        .world_mut()
        .query_filtered::<(&Transform, &Sprite), With<BeaconLight>>();
    let lights = query
        .iter(app.world())
        .map(|(transform, sprite)| (transform.translation, sprite.custom_size))
        .collect::<Vec<_>>();

    assert_eq!(lights.len(), BEACON_COUNT);
    for (translation, size) in lights {
        assert_close(translation.y, -BEACON_MAST.y / 2.0);
        assert_eq!(size, Some(Vec2::splat(BEACON_LIGHT)));
    }
}
