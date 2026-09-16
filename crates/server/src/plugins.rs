use bevy::prelude::*;

use crate::{abilities, network, player, security};

pub struct ServerAppPlugin {
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

/// Configures the server application plugin from operator settings.
///
/// # Errors
///
/// Propagates configuration or I/O errors from reading the server key file.
pub fn configure() -> Result<Option<ServerAppPlugin>, Box<dyn std::error::Error>> {
    Ok(security::configure()?.map(|key| ServerAppPlugin { key }))
}
