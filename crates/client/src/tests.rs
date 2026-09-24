// Client tests drive crate-internal systems and need module-private state, so every area lives
// under this test module instead of a separate integration test target.

mod background;
mod beacons;
mod camera;
mod flame;
#[cfg(not(target_family = "wasm"))]
mod frame;
mod gizmos;
#[cfg(not(target_family = "wasm"))]
mod hud;
mod network;
mod palette;
mod player;
