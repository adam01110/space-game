use std::time::Duration;

use avian2d::prelude::Position;
use bevy::prelude::*;
use lightyear::prelude::ControlledBy;

use space_game_protocol::{BlasterShot, PhaseBeamSegment, Player, PlayerHealth};

use crate::{
    PLAYER_RADIUS,
    phase_beam::{BEAM_LENGTH, BEAM_WIDTH},
};

pub const BULLET_DAMAGE: u8 = 10;

// Damage a phase beam deals per second of contact.
pub const PHASE_BEAM_DAMAGE_PER_SECOND: u8 = 25;

// Hit circle of a shot. The rendered projectile is a 4 x 16 rectangle, so its width is the
// closer approximation, and the circle stays wider than the 20 units a shot covers per tick
// so it cannot tunnel past a player. Public so client development overlays can draw the
// volume damage is resolved against.
pub const SHOT_RADIUS: f32 = 4.0;

/// Weapon damage values, in health points. Replace the resource to retune both
/// weapons at once; `Default` holds the values a session starts with.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageConfig {
    pub bullet: u8,
    pub phase_beam_per_second: u8,
}

impl Default for DamageConfig {
    fn default() -> Self {
        Self {
            bullet: BULLET_DAMAGE,
            phase_beam_per_second: PHASE_BEAM_DAMAGE_PER_SECOND,
        }
    }
}

// Sub-unit beam damage carried between ticks, so a rate below one damage unit per tick still
// lands. Server-only bookkeeping, never replicated.
#[derive(Component, Default)]
pub struct BeamDamageCarry(Duration);

type DamageTargets<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static mut PlayerHealth,
        &'static mut BeamDamageCarry,
        Option<&'static ControlledBy>,
    ),
    With<Player>,
>;

type Beams<'w, 's> =
    Query<'w, 's, (&'static PhaseBeamSegment, Option<&'static ControlledBy>), With<Player>>;

// A player never damages themself with their own weapon. Entities without an owner
// (tests, uncontrolled spawns) are always eligible.
fn same_owner(weapon: Option<&ControlledBy>, target: Option<&ControlledBy>) -> bool {
    matches!((weapon, target), (Some(weapon), Some(target)) if weapon.owner == target.owner)
}

fn circle_hit(a: Vec2, a_radius: f32, b: Vec2, b_radius: f32) -> bool {
    a.distance_squared(b) <= (a_radius + b_radius).powi(2)
}

// Distance to the finite beam segment, not the infinite ray: a target behind the
// muzzle or past the tip is safe.
fn segment_hit(segment: &PhaseBeamSegment, target: Vec2) -> bool {
    let to_target = target - segment.origin;
    let along = to_target.dot(segment.direction).clamp(0.0, BEAM_LENGTH);
    let closest = segment.origin + segment.direction * along;

    circle_hit(closest, BEAM_WIDTH / 2.0, target, PLAYER_RADIUS)
}

// Turn a per-second rate into whole damage units, keeping the fraction for the next tick.
// Whole-unit damage keeps health integral and rollback-friendly.
fn accrue_damage(remainder: &mut Duration, units_per_second: u8, delta: Duration) -> u8 {
    let accrued = remainder.saturating_add(delta.saturating_mul(u32::from(units_per_second)));
    *remainder = Duration::new(0, accrued.subsec_nanos());

    u8::try_from(accrued.as_secs()).unwrap_or(u8::MAX)
}

// Authoritative only: predicted copies of shots and beams never damage anyone.
// The first player hit absorbs the shot, so one projectile can never hit twice.
pub(super) fn apply_bullet_damage(
    mut commands: Commands,
    config: Res<DamageConfig>,
    shots: Query<(Entity, &BlasterShot, Option<&ControlledBy>)>,
    mut targets: DamageTargets,
) {
    for (shot, projectile, shooter) in &shots {
        let hit = targets.iter_mut().find(|(position, _, _, target)| {
            !same_owner(shooter, *target)
                && circle_hit(projectile.position, SHOT_RADIUS, position.0, PLAYER_RADIUS)
        });

        if let Some((_, mut health, _, _)) = hit {
            health.0 = health.0.saturating_sub(config.bullet);
            commands.entity(shot).despawn();
        }
    }
}

// Overlapping beams do not stack: contact is contact, so the rate stays per second.
pub(super) fn apply_phase_beam_damage(
    time: Res<Time<Fixed>>,
    config: Res<DamageConfig>,
    beams: Beams,
    mut targets: DamageTargets,
) {
    for (position, mut health, mut carry, target) in &mut targets {
        let beamed = beams
            .iter()
            .any(|(segment, beam)| !same_owner(beam, target) && segment_hit(segment, position.0));

        if !beamed {
            // Contact ended, so a fresh hit starts its own fraction.
            carry.0 = Duration::ZERO;
            continue;
        }

        let damage = accrue_damage(&mut carry.0, config.phase_beam_per_second, time.delta());
        health.0 = health.0.saturating_sub(damage);
    }
}
