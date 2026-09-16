mod abilities;
mod arena;
mod blasters;
mod movement;
pub mod phase_beam;
mod physics;
mod player;
mod plugins;

pub use arena::{
    ARENA_GROWTH_MULTIPLIER, ARENA_RESIZE_SPEED, BASE_ARENA_RADIUS, advance_arena, arena_radius,
    contain_circle, contain_predicted_bodies, install_arena_interpolation,
};
pub use player::{PLAYER_RADIUS, PlayerBundle};
pub use plugins::{
    ClientSimulationPlugin, ClientSimulationSystems, GamePlugin, SERVER_UPS, ServerSimulationPlugin,
};
