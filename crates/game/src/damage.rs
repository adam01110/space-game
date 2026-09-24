use std::time::Duration;

use avian2d::prelude::Position;
use bevy::prelude::*;
use lightyear::prelude::ControlledBy;

use space_game_protocol::{
    Asteroid, AsteroidHealth, BlasterShot, CircleBody, PhaseBeamSegment, Player, PlayerHealth,
};

use crate::phase_beam::{BEAM_LENGTH, BEAM_WIDTH};

pub const BULLET_DAMAGE: u8 = 10;
pub const PHASE_BEAM_DAMAGE_PER_SECOND: u8 = 20;
pub const SHOT_RADIUS: f32 = 6.0;

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

// Server-only fractional beam damage. An asteroid and a player use the same contact clock.
#[derive(Component, Default)]
pub struct BeamDamageCarry(Duration);

type DamageTargets<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static CircleBody,
        Option<&'static mut PlayerHealth>,
        Option<&'static mut AsteroidHealth>,
        &'static mut BeamDamageCarry,
        Option<&'static ControlledBy>,
    ),
    Or<(With<Player>, With<Asteroid>)>,
>;

type Beams<'w, 's> =
    Query<'w, 's, (&'static PhaseBeamSegment, Option<&'static ControlledBy>), With<Player>>;

fn same_owner(weapon: Option<&ControlledBy>, target: Option<&ControlledBy>) -> bool {
    matches!((weapon, target), (Some(weapon), Some(target)) if weapon.owner == target.owner)
}

fn circle_hit(a: Vec2, a_radius: f32, b: Vec2, b_radius: f32) -> bool {
    a.distance_squared(b) <= (a_radius + b_radius).powi(2)
}

fn segment_hit(segment: &PhaseBeamSegment, target: Vec2, radius: f32) -> bool {
    let to_target = target - segment.origin;
    let along = to_target.dot(segment.direction).clamp(0.0, BEAM_LENGTH);
    let closest = segment.origin + segment.direction * along;
    circle_hit(closest, BEAM_WIDTH / 2.0, target, radius)
}

fn accrue_damage(remainder: &mut Duration, units_per_second: u8, delta: Duration) -> u8 {
    let accrued = remainder.saturating_add(delta.saturating_mul(u32::from(units_per_second)));
    *remainder = Duration::new(0, accrued.subsec_nanos());
    u8::try_from(accrued.as_secs()).unwrap_or(u8::MAX)
}

// Weapons damage only their target; mutual damage is reserved for physical contact.
fn deal_damage(
    player: Option<Mut<PlayerHealth>>,
    asteroid: Option<Mut<AsteroidHealth>>,
    amount: u8,
) {
    if let Some(mut health) = asteroid {
        health.0 = health.0.saturating_sub(amount);
    } else if let Some(mut health) = player {
        health.0 = health.0.saturating_sub(amount);
    }
}

pub(super) fn apply_bullet_damage(
    mut commands: Commands,
    config: Res<DamageConfig>,
    shots: Query<(Entity, &BlasterShot, Option<&ControlledBy>)>,
    mut targets: DamageTargets,
) {
    for (shot, projectile, shooter) in &shots {
        let hit = targets
            .iter_mut()
            .find(|(position, body, player, asteroid, _, target)| {
                !same_owner(shooter, *target)
                    && (player.as_ref().is_some_and(|h| h.0 > 0)
                        || asteroid.as_ref().is_some_and(|h| h.0 > 0))
                    && circle_hit(projectile.position, SHOT_RADIUS, position.0, body.radius)
            })
            .map(|(_, _, player, asteroid, _, _)| deal_damage(player, asteroid, config.bullet));
        if hit.is_some() {
            commands.entity(shot).despawn();
        }
    }
}

pub(super) fn apply_phase_beam_damage(
    time: Res<Time<Fixed>>,
    config: Res<DamageConfig>,
    beams: Beams,
    mut targets: DamageTargets,
) {
    for (position, body, player, asteroid, mut carry, target) in &mut targets {
        if player.as_ref().is_some_and(|h| h.0 == 0) || asteroid.as_ref().is_some_and(|h| h.0 == 0)
        {
            carry.0 = Duration::ZERO;
            continue;
        }
        let hitting = beams.iter().find(|(segment, beam)| {
            !same_owner(*beam, target) && segment_hit(segment, position.0, body.radius)
        });
        let Some(_) = hitting else {
            carry.0 = Duration::ZERO;
            continue;
        };
        let damage = accrue_damage(&mut carry.0, config.phase_beam_per_second, time.delta());
        deal_damage(player, asteroid, damage);
    }
}

pub(super) fn despawn_destroyed_asteroids(
    mut commands: Commands,
    asteroids: Query<(Entity, &AsteroidHealth), With<Asteroid>>,
) {
    for (entity, health) in &asteroids {
        if health.0 == 0 {
            commands.entity(entity).despawn();
        }
    }
}
