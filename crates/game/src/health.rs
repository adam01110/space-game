use std::time::Duration;

use bevy::prelude::*;

use space_game_protocol::{Player, PlayerHealth};

pub const HEALTH_REGEN_PER_SECOND: u8 = 5;

// Server-only elapsed time since the last health point; each player starts their own clock.
#[derive(Component, Default)]
pub(crate) struct HealthRegenCarry(Duration);

pub(super) fn regenerate_health(
    time: Res<Time<Fixed>>,
    mut players: Query<(&mut PlayerHealth, &mut HealthRegenCarry), With<Player>>,
) {
    for (mut health, mut carry) in &mut players {
        if health.0 == 0 || health.0 >= PlayerHealth::FULL {
            carry.0 = Duration::ZERO;
            continue;
        }

        let elapsed = carry.0.saturating_add(
            time.delta()
                .saturating_mul(u32::from(HEALTH_REGEN_PER_SECOND)),
        );
        let points = u8::try_from(elapsed.as_secs()).unwrap_or(u8::MAX);

        carry.0 = Duration::new(0, elapsed.subsec_nanos());
        health.0 = health.0.saturating_add(points).min(PlayerHealth::FULL);
    }
}
