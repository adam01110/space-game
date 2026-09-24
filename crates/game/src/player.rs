use avian2d::prelude::*;
use bevy::prelude::*;

use space_game_protocol::{
    BlasterReload, BlasterTrigger, CircleBody, Player, PlayerBlasters, PlayerBoost, PlayerHealth,
    PlayerPhaseBeam,
};

use crate::{damage::BeamDamageCarry, health::HealthRegenCarry};

pub const PLAYER_RADIUS: f32 = 23.4;

#[derive(Bundle)]
pub struct PlayerBundle {
    blasters: PlayerBlasters,
    blaster_trigger: BlasterTrigger,
    blaster_reload: BlasterReload,
    beam_damage_carry: BeamDamageCarry,
    boost: PlayerBoost,
    health: PlayerHealth,
    health_regen_carry: HealthRegenCarry,
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
            blaster_reload: BlasterReload::default(),
            beam_damage_carry: BeamDamageCarry::default(),
            boost: PlayerBoost::default(),
            health: PlayerHealth::default(),
            health_regen_carry: HealthRegenCarry::default(),
            phase_beam: PlayerPhaseBeam::default(),
            player: Player,
            circle: CircleBody::dynamic(PLAYER_RADIUS),
            position: Position(position),
            rotation: Rotation::default(),
        }
    }
}
