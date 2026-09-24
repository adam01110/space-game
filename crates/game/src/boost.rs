use std::time::Duration;

use space_game_protocol::PlayerBoost;

const DRAIN_PER_SECOND: u8 = 2;
pub(super) const SPEED_MULTIPLIER: f32 = 1.5;

pub(super) fn use_boost(charge: &mut PlayerBoost, held: bool, delta: Duration) -> bool {
    if !held {
        return false;
    }

    charge.0.drain(DRAIN_PER_SECOND, delta)
}
