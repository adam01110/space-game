mod abilities;
mod arena;
mod blasters;
mod movement;
pub mod phase_beam;
mod physics;
mod player;
mod plugins;

pub use arena::{ARENA_GROWTH_MULTIPLIER, ARENA_RESIZE_SPEED, BASE_ARENA_RADIUS, arena_radius};
pub use player::{PLAYER_RADIUS, PlayerBundle};
pub use plugins::{ClientSimulationPlugin, GamePlugin, SERVER_UPS, ServerSimulationPlugin};
