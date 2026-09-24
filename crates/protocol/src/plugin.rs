use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    ArenaBoundary, Asteroid, AsteroidHealth, BlasterReload, BlasterShot, BlasterTrajectory,
    BlasterTrigger, CircleBody, PhaseBeamSegment, Player, PlayerBlasters, PlayerBoost,
    PlayerHealth, PlayerIdentity, PlayerInput, PlayerPhaseBeam,
};

// Registers replicated components and the shared player input type.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::native::InputPlugin::<PlayerInput> {
            config: input::InputConfig {
                // All colliding players run on the prediction timeline, including remote ones.
                rebroadcast_inputs: true,
                // About 250 ms at 60 packets/s versus the default 5 (~83 ms); more redundant
                // history tolerates short loss bursts at the cost of larger client packets and
                // server rebroadcast traffic.
                packet_redundancy: 15,
                ..default()
            },
        });

        app.component::<ArenaBoundary>().replicate().predict();
        app.component::<Asteroid>().replicate();
        app.component::<AsteroidHealth>().replicate().predict();
        app.component::<Player>().replicate();
        app.component::<PlayerIdentity>().replicate();
        app.component::<PlayerBlasters>().replicate().predict();
        app.component::<BlasterTrigger>().replicate().predict();
        app.component::<BlasterReload>().replicate().predict();
        app.component::<BlasterShot>().replicate().predict();
        app.component::<BlasterTrajectory>().replicate().predict();
        app.component::<PlayerBoost>().replicate().predict();
        app.component::<PlayerHealth>().replicate().predict();
        app.component::<PlayerPhaseBeam>().replicate().predict();
        // Presence is authoritative for remote beam presentation and predicted locally so the
        // controlling player skips a server round trip.
        app.component::<PhaseBeamSegment>().replicate().predict();
        // Configuration is immutable during predicted simulation; runtime shape changes must gain
        // prediction/rollback support before being used as gameplay.
        app.component::<CircleBody>().replicate();
        // GamePlugin installs the Avian pose/velocity protocol on both client and server.
    }
}
