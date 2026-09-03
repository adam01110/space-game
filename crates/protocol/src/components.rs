use bevy::{math::Curve, prelude::*};
use serde::{Deserialize, Serialize};

// Marks an entity as a player-controlled ship.
#[derive(Component, Serialize, Deserialize)]
pub struct Player;

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
