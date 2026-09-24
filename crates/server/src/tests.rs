// Server tests build headless apps around the crate's own connection handling, so every area lives
// under this test module instead of a separate integration test target.

mod abilities;
mod asteroids;
mod player;
