// The hitbox overlays exist only in development builds, so their tests do too.
#![cfg(feature = "dev")]

use bevy::prelude::*;

use space_game_game::{
    SHOT_RADIUS,
    phase_beam::{BEAM_LENGTH, BEAM_WIDTH},
};
use space_game_protocol::{BlasterShot, PhaseBeamSegment};

use crate::gizmos::{beam_hitbox, shot_hitbox};

// Capsule geometry runs through the trigonometry that builds its rotation, so comparisons that
// touch it keep a tolerance while the ones that do not stay exact.
const TOLERANCE: f32 = 1e-3;

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "{what}: expected {expected}, got {actual}"
    );
}

fn assert_at(actual: Vec2, expected: Vec2, what: &str) {
    assert!(
        actual.distance(expected) < TOLERANCE,
        "{what}: expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn a_shot_hitbox_is_the_damage_circle_around_the_shot() {
    let shot = BlasterShot {
        position: Vec2::new(12.0, -34.0),
        ticks_left: 3,
    };

    let (center, radius) = shot_hitbox(&shot);

    assert_at(center, shot.position, "shot hitbox centre");
    assert_close(radius, SHOT_RADIUS, "shot hitbox radius");
}

// The capsule has to cover the volume damage uses, which is the circle of half the beam width
// swept along the segment: a cylinder of `BEAM_LENGTH` capped at the muzzle and at the tip.
#[test]
fn a_beam_hitbox_covers_the_damage_segment() {
    let segment = PhaseBeamSegment {
        origin: Vec2::new(10.0, -5.0),
        direction: Vec2::new(1.0, 2.0),
    };
    let direction = segment.direction.normalize();

    let (capsule, length, radius) = beam_hitbox(&segment);
    let axis = capsule.rotation * Vec2::Y;
    let muzzle = capsule.translation - axis * (BEAM_LENGTH / 2.0);
    let tip = capsule.translation + axis * (BEAM_LENGTH / 2.0);

    assert_close(length, BEAM_LENGTH, "beam hitbox length");
    assert_close(radius, BEAM_WIDTH / 2.0, "beam hitbox radius");
    assert_at(axis, direction, "beam hitbox axis");
    assert_at(muzzle, segment.origin, "beam hitbox muzzle");
    assert_at(
        tip,
        segment.origin + direction * BEAM_LENGTH,
        "beam hitbox tip",
    );
}

// A beam that has lost its direction, for instance one the simulation has not posed yet, still
// has to produce a drawable capsule instead of a NaN isometry.
#[test]
fn a_directionless_beam_hitbox_stays_finite() {
    let segment = PhaseBeamSegment {
        origin: Vec2::new(-3.0, 8.0),
        direction: Vec2::ZERO,
    };

    let (capsule, _, radius) = beam_hitbox(&segment);

    assert!(capsule.translation.is_finite(), "centre {capsule:?}");
    assert!(radius.is_finite(), "radius {radius}");
}
