// Client tests drive crate-internal systems and need access to module-private state, so every
// area lives under this test module instead of a separate integration test target.

mod background;
mod camera;
mod confirmed;
mod focus;
mod input;
mod palette;
