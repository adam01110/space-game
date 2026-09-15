use bevy::{ecs::entity::MapEntities, prelude::*};
use serde::{Deserialize, Serialize};

// Input sent by a client for one simulation tick.
#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct PlayerInput {
    // Desired world-space movement direction before normalization.
    pub movement: Vec2,
    // World-space direction the ship should face.
    pub aim: Vec2,
    // Cumulative clicks survive repeated/missing simulation ticks without automatic fire.
    pub blaster_clicks: u32,
    // Cumulative R presses, consumed once even across repeated simulation ticks.
    pub blaster_reload_requests: u32,
    // Held state; the visual beam disappears when Space is released.
    pub phase_beam: bool,
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self {
            movement: Vec2::ZERO,
            aim: Vec2::Y,
            blaster_clicks: 0,
            blaster_reload_requests: 0,
            phase_beam: false,
        }
    }
}

impl MapEntities for PlayerInput {
    fn map_entities<M: EntityMapper>(&mut self, _entity_mapper: &mut M) {}
}
