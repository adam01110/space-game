use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{Player, PlayerHeading, PlayerInput, PlayerPosition};

// Registers every type that crosses the network boundary.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<PlayerInput>::default());

        app.component::<Player>().replicate();
        app.component::<PlayerPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();
        app.component::<PlayerHeading>()
            .replicate()
            .predict()
            .add_linear_interpolation();
    }
}
