use bevy::{math::Curve, prelude::*};
use serde::{Deserialize, Serialize};

// Marks an entity as a player-controlled ship.
#[derive(Component, Serialize, Deserialize)]
pub struct Player;

// Remaining cooldown time for a player's blasters, in simulation ticks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerBlasters(pub u16);

// Remaining cooldown time for a player's boost, in simulation ticks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerBoost(pub u16);

// Current health of a player.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealth(pub u16);

impl Default for PlayerHealth {
    fn default() -> Self {
        Self(100)
    }
}

// Remaining cooldown time for a player's phase beam, in simulation ticks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerPhaseBeam(pub u16);

// Authoritative position of a player in world units.
#[derive(Component, Clone, Debug, Default, Deref, PartialEq, Serialize, Deserialize)]
pub struct PlayerPosition(pub Vec2);

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(start.0.lerp(end.0, t))
        })
    }
}

// Counter-clockwise ship rotation in radians. Zero points along local +Y.
#[derive(Component, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerHeading(pub f32);

impl Ease for PlayerHeading {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerHeading(start.0 + (end.0 - start.0) * t)
        })
    }
}
