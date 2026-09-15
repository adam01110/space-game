use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Fixed-point ability charge: 100 internal units equal one UI percentage point.
// Simulation retains this precision; presentation can scale to 0–100 separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AbilityCharge {
    units: u16,
    // Scaled time: one second represents one charge unit. Fractional progress
    // travels with charge so prediction rollback restores both.
    regen_remainder: Duration,
}

impl AbilityCharge {
    pub const FULL: u16 = 10_000;
    pub const UNITS_PER_PERCENT: u16 = 100;

    #[must_use]
    pub const fn units(self) -> u16 {
        self.units
    }

    // Rates are internal units per second. Keep sub-unit progress across fixed ticks
    // without floating-point rounding, and discard overflow at full charge.
    pub fn regenerate(&mut self, units_per_second: u16, delta: Duration) {
        let accrued = self
            .regen_remainder
            .saturating_add(delta.saturating_mul(u32::from(units_per_second)));

        let recovered = accrued.as_secs();
        let missing = Self::FULL - self.units;

        if recovered >= u64::from(missing) {
            self.units = Self::FULL;
            self.regen_remainder = Duration::ZERO;
        } else {
            self.units += recovered as u16;
            self.regen_remainder = accrued - Duration::from_secs(recovered);
        }
    }

    // Spend charge on up to `requested` activations, returning the affordable count.
    pub fn spend(&mut self, cost: u16, requested: u32) -> u16 {
        if cost == 0 {
            return 0;
        }
        let count = u16::try_from(requested)
            .unwrap_or(u16::MAX)
            .min(self.units / cost);
        self.units -= count * cost;
        count
    }
}

impl Default for AbilityCharge {
    fn default() -> Self {
        Self {
            units: Self::FULL,
            regen_remainder: Duration::ZERO,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBlasters(pub AbilityCharge);

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBoost(pub AbilityCharge);

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerPhaseBeam(pub AbilityCharge);
