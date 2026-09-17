// Game tests drive the simulation plugins from inside the crate, so every area lives under this
// test module instead of a separate integration test target.

mod arena;
mod collision;
mod movement;
mod phase_beam;
mod support;
