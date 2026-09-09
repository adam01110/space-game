use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    Player, PlayerBlasters, PlayerBoost, PlayerHeading, PlayerHealth, PlayerInput, PlayerPhaseBeam,
    PlayerPosition,
};

// Registers player input and configures how each player component is synchronized.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<PlayerInput>::default());

        app.component::<Player>().replicate();
        app.component::<PlayerBlasters>().replicate().predict();
        app.component::<PlayerBoost>().replicate().predict();
        app.component::<PlayerHealth>().replicate().predict();
        app.component::<PlayerPhaseBeam>().replicate().predict();
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
