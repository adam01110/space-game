use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Marks an entity as a player-controlled ship.
#[derive(Component, Serialize, Deserialize)]
pub struct Player;

// Remaining cooldown time for a player's blasters, in simulation ticks.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBlasters(pub u16);

// Remaining cooldown time for a player's boost, in simulation ticks.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerBoost(pub u16);

// Current health of a player.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealth(pub u16);

impl Default for PlayerHealth {
    fn default() -> Self {
        Self(100)
    }
}

// Remaining cooldown time for a player's phase beam, in simulation ticks.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerPhaseBeam(pub u16);

/// Shape and motion configuration shared by all solid world objects, not just players.
///
/// Radii are gameplay approximations in world units, independent of sprite geometry.
/// Networked dynamic objects must be predicted by every client that simulates them.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircleBody {
    pub radius: f32,
    pub motion: BodyMotion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BodyMotion {
    Dynamic,
    Static,
}

impl CircleBody {
    #[must_use]
    pub const fn dynamic(radius: f32) -> Self {
        Self {
            radius,
            motion: BodyMotion::Dynamic,
        }
    }

    #[must_use]
    pub const fn fixed(radius: f32) -> Self {
        Self {
            radius,
            motion: BodyMotion::Static,
        }
    }
}
