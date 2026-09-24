// Development-only world-space overlays: the volumes the simulation resolves weapon damage
// against, plus the palette swatches those overlays and the arena border draw with. Gameplay never
// reads this module, so only the physics debug build compiles it.
#[cfg(feature = "dev")]
use avian2d::prelude::PhysicsGizmos;
#[cfg(feature = "dev")]
use bevy::prelude::*;
#[cfg(feature = "dev")]
use lightyear::prelude::Predicted;

#[cfg(feature = "dev")]
use space_game_game::{
    SHOT_RADIUS,
    phase_beam::{BEAM_LENGTH, BEAM_WIDTH},
};
#[cfg(feature = "dev")]
use space_game_protocol::{BlasterShot, PhaseBeamSegment};

#[cfg(feature = "dev")]
use crate::palette::Palette;

// Overlays are drawn on the same native-resolution debug layer as the arena and backdrop chunk
// outlines. Every swatch comes from the game palette, so diagnostics stay part of the game's
// picture instead of wearing gizmo default colours.

// Outline of the server-owned play area, drawn by `arena::draw_arena`.
#[cfg(feature = "dev")]
pub(super) const ARENA_BORDER: Color = Palette::Olive.color();

// Hit circle of a blaster shot: wider than the Citron projectile sprite drawn inside it.
#[cfg(feature = "dev")]
pub(super) const SHOT_HITBOX: Color = Palette::Sand.color();

// Hit volume of a phase beam, which covers the whole Mint beam sprite and its round caps.
#[cfg(feature = "dev")]
pub(super) const BEAM_HITBOX: Color = Palette::Teal.color();

// The circle a shot damages with: its own position, so a hit only needs the two radii.
#[cfg(feature = "dev")]
pub(super) const fn shot_hitbox(shot: &BlasterShot) -> (Vec2, f32) {
    (shot.position, SHOT_RADIUS)
}

// The capsule a beam damages with: `BEAM_WIDTH` swept along the finite segment from the muzzle to
// `BEAM_LENGTH` along the firing direction, whose round ends match the way the damage query clamps
// the segment. Returned as the isometry of a capsule whose axis is +Y, its length, and its radius.
#[cfg(feature = "dev")]
pub(super) fn beam_hitbox(segment: &PhaseBeamSegment) -> (Isometry2d, f32, f32) {
    let direction = segment.direction.normalize_or_zero();
    let center = segment.origin + direction * (BEAM_LENGTH / 2.0);
    let axis = Rot2::radians(direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2);

    (Isometry2d::new(center, axis), BEAM_LENGTH, BEAM_WIDTH / 2.0)
}

// One entity carries `BlasterShot`, predicted or replicated, so this outlines exactly the
// projectiles that are drawn.
#[cfg(feature = "dev")]
pub(super) fn draw_shot_hitboxes(shots: Query<&BlasterShot>, mut gizmos: Gizmos<PhysicsGizmos>) {
    for shot in &shots {
        let (center, radius) = shot_hitbox(shot);
        gizmos.circle_2d(center, radius, SHOT_HITBOX);
    }
}

// Every beam is predicted on this client, local and remote alike, so the drawn volume is what this
// client simulates and the server resolves damage with.
#[cfg(feature = "dev")]
pub(super) fn draw_beam_hitboxes(
    beams: Query<&PhaseBeamSegment, With<Predicted>>,
    mut gizmos: Gizmos<PhysicsGizmos>,
) {
    for beam in &beams {
        let (isometry, length, radius) = beam_hitbox(beam);
        gizmos.primitive_2d(&Capsule2d::new(radius, length), isometry, BEAM_HITBOX);
    }
}
