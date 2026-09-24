// Game tests drive the simulation plugins from inside the crate, so every area lives under this
// test module instead of a separate integration test target.

mod arena;
mod blasters;
mod collision;
mod contact_damage;
mod damage;
mod health;
mod movement;
mod phase_beam;
mod support;
