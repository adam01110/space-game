use bevy::prelude::*;

use project_protocol::{
    Player, PlayerBlasters, PlayerBoost, PlayerHeading, PlayerHealth, PlayerPhaseBeam,
    PlayerPosition,
};

#[derive(Bundle)]
pub struct PlayerBundle {
    blasters: PlayerBlasters,
    boost: PlayerBoost,
    heading: PlayerHeading,
    health: PlayerHealth,
    phase_beam: PlayerPhaseBeam,
    player: Player,
    position: PlayerPosition,
}

impl PlayerBundle {
    pub fn new(position: Vec2) -> Self {
        Self {
            blasters: PlayerBlasters::default(),
            boost: PlayerBoost::default(),
            heading: PlayerHeading::default(),
            health: PlayerHealth::default(),
            phase_beam: PlayerPhaseBeam::default(),
            player: Player,
            position: PlayerPosition(position),
        }
    }
}
