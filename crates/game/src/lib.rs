mod abilities;
mod blasters;
mod movement;
pub mod phase_beam;
mod physics;
mod player;
mod plugins;

pub use player::{PLAYER_RADIUS, PlayerBundle};
pub use plugins::{ClientSimulationPlugin, GamePlugin, SERVER_UPS, ServerSimulationPlugin};
