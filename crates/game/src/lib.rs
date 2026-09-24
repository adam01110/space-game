mod abilities;
mod arena;
mod blasters;
mod boost;
mod damage;
mod health;
mod movement;
pub mod phase_beam;
mod physics;
mod player;
mod plugins;

#[cfg(test)]
mod tests;

pub use arena::{
    ARENA_GROWTH_MULTIPLIER, ARENA_RESIZE_SPEED, BASE_ARENA_RADIUS, advance_arena, arena_radius,
    contain_circle, contain_predicted_bodies, install_arena_interpolation,
};
pub use damage::{BULLET_DAMAGE, DamageConfig, PHASE_BEAM_DAMAGE_PER_SECOND, SHOT_RADIUS};
pub use health::HEALTH_REGEN_PER_SECOND;
pub use player::{PLAYER_RADIUS, PlayerBundle};
pub use plugins::{ClientSimulationPlugin, GamePlugin, SERVER_UPS, ServerSimulationPlugin};
