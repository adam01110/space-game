use bevy::{ecs::entity::MapEntities, math::Curve, prelude::*};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub const PRIVATE_KEY: [u8; 32] = [0; 32];
pub const PROTOCOL_ID: u64 = 0;
pub const SERVER_PORT: u16 = 5000;

// Marks an entity as a player-controlled ship.
#[derive(Component, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Player;

// Authoritative position of a player in world units.
#[derive(
    Component, Clone, Copy, Debug, Default, Deref, DerefMut, PartialEq, Serialize, Deserialize,
)]
pub struct PlayerPosition(pub Vec2);

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(start.0.lerp(end.0, t))
        })
    }
}

// Counter-clockwise ship rotation in radians. Zero points along local +Y.
#[derive(
    Component, Clone, Copy, Debug, Default, Deref, DerefMut, PartialEq, Serialize, Deserialize,
)]
pub struct PlayerHeading(pub f32);

impl Ease for PlayerHeading {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerHeading(start.0 + (end.0 - start.0) * t)
        })
    }
}

// Input sent by a client for one simulation tick.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct PlayerInput {
    // World-space movement axes, each in the range `-1.0..=1.0`.
    pub movement: Vec2,
    // World-space direction in which the ship should turn.
    pub aim: Vec2,
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self {
            movement: Vec2::ZERO,
            aim: Vec2::Y,
        }
    }
}

impl MapEntities for PlayerInput {
    fn map_entities<M: EntityMapper>(&mut self, _entity_mapper: &mut M) {}
}

// Registers every type that crosses the network boundary.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<PlayerInput>::default());

        app.component::<Player>().replicate();
        app.component::<PlayerPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();
        app.component::<PlayerHeading>()
            .replicate()
            .predict()
            .add_linear_interpolation();
    }
}
