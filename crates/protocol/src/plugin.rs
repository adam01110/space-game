use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    BlasterReload, BlasterShot, BlasterTrigger, CircleBody, Player, PlayerBlasters, PlayerBoost,
    PlayerHealth, PlayerInput, PlayerPhaseBeam,
};

// Registers replicated components and the shared player input type.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<PlayerInput> {
            config: input::InputConfig {
                // All colliding players run on the prediction timeline, including remote ones.
                rebroadcast_inputs: true,
                ..default()
            },
        });

        app.component::<Player>().replicate();
        app.component::<PlayerBlasters>().replicate().predict();
        app.component::<BlasterTrigger>().replicate().predict();
        app.component::<BlasterReload>().replicate().predict();
        app.component::<BlasterShot>().replicate();
        app.component::<PlayerBoost>().replicate().predict();
        app.component::<PlayerHealth>().replicate().predict();
        app.component::<PlayerPhaseBeam>().replicate().predict();
        // Configuration is immutable during predicted simulation. Runtime shape changes must
        // gain prediction/rollback support before being used as gameplay.
        app.component::<CircleBody>().replicate();
        // GamePlugin installs the Avian pose/velocity protocol on both client and server.
    }
}
