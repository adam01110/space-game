use bevy::{ecs::entity::MapEntities, prelude::*};
use serde::{Deserialize, Serialize};

// Input sent by a client for one simulation tick.
#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct PlayerInput {
    pub movement: Vec2,
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
