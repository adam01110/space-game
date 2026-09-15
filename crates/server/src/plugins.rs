use bevy::prelude::*;

use crate::{abilities, network, player, security};

pub(super) struct ServerAppPlugin {
    key: security::ServerKey,
}

impl Plugin for ServerAppPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(security::ServerKey(self.key.0))
            .add_plugins((
                abilities::ServerAbilitiesPlugin,
                network::ServerNetworkPlugin,
                player::ServerPlayerPlugin,
            ));
    }
}

pub(super) fn configure() -> Result<Option<ServerAppPlugin>, Box<dyn std::error::Error>> {
    Ok(security::configure()?.map(|key| ServerAppPlugin { key }))
}
