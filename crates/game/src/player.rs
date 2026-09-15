use avian2d::prelude::*;
use bevy::prelude::*;

use project_protocol::{
    BlasterTrigger, CircleBody, Player, PlayerBlasters, PlayerBoost, PlayerHealth, PlayerPhaseBeam,
};

pub const PLAYER_RADIUS: f32 = 18.0;

#[derive(Bundle)]
pub struct PlayerBundle {
    blasters: PlayerBlasters,
    blaster_trigger: BlasterTrigger,
    boost: PlayerBoost,
    health: PlayerHealth,
    phase_beam: PlayerPhaseBeam,
    player: Player,
    circle: CircleBody,
    position: Position,
    rotation: Rotation,
}

impl PlayerBundle {
    #[must_use]
    pub fn new(position: Vec2) -> Self {
        Self {
            blasters: PlayerBlasters::default(),
            blaster_trigger: BlasterTrigger::default(),
            boost: PlayerBoost::default(),
            health: PlayerHealth::default(),
            phase_beam: PlayerPhaseBeam::default(),
            player: Player,
            circle: CircleBody::dynamic(PLAYER_RADIUS),
            position: Position(position),
            rotation: Rotation::default(),
        }
    }
}
