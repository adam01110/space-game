use bevy::prelude::*;
use project_protocol::{Player, PlayerHeading, PlayerPosition};

#[derive(Bundle)]
pub struct PlayerBundle {
    heading: PlayerHeading,
    player: Player,
    position: PlayerPosition,
}

impl PlayerBundle {
    pub fn new(position: Vec2) -> Self {
        Self {
            heading: PlayerHeading::default(),
            player: Player,
            position: PlayerPosition(position),
        }
    }
}
