use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Fixed-point ability charge: 100 internal units equal one UI percentage point.
// Simulation retains this precision; presentation can scale to 0–100 separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AbilityCharge(u16);

impl AbilityCharge {
    pub const FULL: u16 = 10_000;
    pub const UNITS_PER_PERCENT: u16 = 100;

    #[must_use]
    pub const fn units(self) -> u16 {
        self.0
    }

    // Spend charge on up to `requested` activations, returning the affordable count.
    // There is deliberately no automatic regeneration.
    pub fn spend(&mut self, cost: u16, requested: u32) -> u16 {
        if cost == 0 {
            return 0;
        }
        let count = u16::try_from(requested)
            .unwrap_or(u16::MAX)
            .min(self.0 / cost);
        self.0 -= count * cost;
        count
    }
}

impl Default for AbilityCharge {
    fn default() -> Self {
        Self(Self::FULL)
    }
}

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBlasters(pub AbilityCharge);

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBoost(pub AbilityCharge);

#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerPhaseBeam(pub AbilityCharge);
