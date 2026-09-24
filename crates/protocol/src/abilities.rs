use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Ability charge measured in whole percentage points.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AbilityCharge {
    units: u8,
    regen_remainder: Duration,
    drain_remainder: Duration,
}

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBlasters(pub AbilityCharge);

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBoost(pub AbilityCharge);

impl Default for PlayerBoost {
    fn default() -> Self {
        Self(AbilityCharge {
            units: 50,
            ..AbilityCharge::default()
        })
    }
}

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerPhaseBeam(pub AbilityCharge);

impl AbilityCharge {
    pub const FULL: u8 = 100;

    #[must_use]
    pub const fn units(self) -> u8 {
        self.units
    }

    pub fn regenerate(&mut self, units_per_second: u8, delta: Duration) {
        let accrued = self
            .regen_remainder
            .saturating_add(delta.saturating_mul(u32::from(units_per_second)));

        let recovered = u8::try_from(accrued.as_secs()).unwrap_or(u8::MAX);
        let missing = Self::FULL - self.units;

        match recovered >= missing {
            true => {
                self.units = Self::FULL;
                self.regen_remainder = Duration::ZERO;
            }
            false => {
                self.units += recovered;
                self.regen_remainder = Duration::new(0, accrued.subsec_nanos());
            }
        }
    }

    // Drain a per-second rate while charge remains; returns whether the ability can be active for
    // this tick, including the tick that exhausts it.
    pub fn drain(&mut self, units_per_second: u8, delta: Duration) -> bool {
        if self.units == 0 {
            self.drain_remainder = Duration::ZERO;
            return false;
        }

        let accrued = self
            .drain_remainder
            .saturating_add(delta.saturating_mul(u32::from(units_per_second)));
        let consumed = u8::try_from(accrued.as_secs()).unwrap_or(u8::MAX);

        match consumed >= self.units {
            true => {
                self.units = 0;
                self.drain_remainder = Duration::ZERO;
            }
            false => {
                self.units -= consumed;
                self.drain_remainder = Duration::new(0, accrued.subsec_nanos());
            }
        }

        true
    }

    // Spend charge on up to `requested` activations, returning the affordable count.
    pub fn spend(&mut self, cost: u8, requested: u8) -> u8 {
        if cost == 0 {
            return 0;
        }

        let count = requested.min(self.units / cost);
        self.units -= count * cost;
        count
    }
}

impl Default for AbilityCharge {
    fn default() -> Self {
        Self {
            units: Self::FULL,
            regen_remainder: Duration::ZERO,
            drain_remainder: Duration::ZERO,
        }
    }
}
