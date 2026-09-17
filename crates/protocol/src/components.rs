use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Server-owned circular play area centered at the world origin. Predicted with rollback
// so population changes use the authoritative radius, never a client's visible player count.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArenaBoundary {
    pub radius: f32,
    pub target_radius: f32,
}

impl ArenaBoundary {
    #[must_use]
    pub const fn new(radius: f32) -> Self {
        Self {
            radius,
            target_radius: radius,
        }
    }
}

// Marks an entity as a player-controlled ship.
#[derive(Component, Serialize, Deserialize)]
pub struct Player;

// Server-assigned identity used to distinguish deterministic spawns from different players.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerIdentity(pub u64);

// Last processed click counter, replicated for deterministic prediction and rollback.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlasterTrigger(pub u8);

// Reload progress and its consumed input counter must roll back together.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlasterReload {
    pub remaining: Duration,
    pub requests: u8,
}

// Non-solid, server-authoritative projectile. Lifetime is measured in simulation ticks.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlasterShot {
    pub position: Vec2,
    pub direction: Vec2,
    pub ticks_left: u8,
}

// World-space phase beam state produced by the authoritative and predicted simulations.
// Component presence means the beam is active.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseBeamSegment {
    pub origin: Vec2,
    pub direction: Vec2,
}

// Current health of a player.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealth(pub u8);

impl Default for PlayerHealth {
    fn default() -> Self {
        Self(100)
    }
}

// Shape and motion configuration shared by all solid world objects, not just players.
// Radii are gameplay approximations in world units, independent of sprite geometry.
// Networked dynamic objects must be predicted by every client that simulates them.
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
