use bevy::prelude::*;
use lightyear::prelude::{Controlled, input::native::InputMarker};

use space_game_protocol::{Player, PlayerHealth, PlayerInput};

use super::{GuestConnection, connection::retire_session};

type LocalPlayer = (
    With<Player>,
    With<Controlled>,
    With<InputMarker<PlayerInput>>,
);

pub(crate) fn install(app: &mut App) {
    // Retiring despawns ships that asset-driven systems may still hold queued commands for: the SVG
    // rasteriser inserts a sprite into the entity it rasterised, and an entity despawned before that
    // command is flushed panics the browser client with `unreachable`. Retiring in the last schedule
    // of the frame lets every queue of this frame flush first.
    app.add_systems(Last, end_dead_session);
}

// Health reaches zero in the authoritative simulation, so the client only reacts. Retiring
// the session leaves the server to despawn the destroyed player.
fn end_dead_session(
    mut commands: Commands,
    mut connection: ResMut<GuestConnection>,
    players: Query<&PlayerHealth, LocalPlayer>,
    mut ended: Local<bool>,
) {
    let destroyed = players.iter().any(|health| health.0 == 0);

    if !destroyed {
        *ended = false;
    } else if !*ended {
        *ended = true;
        warn!("Player destroyed; returning to the menu");
        retire_session(&mut commands, &mut connection);
        // Each menu owns the next session: the framework menu is rebuilt here, the browser menu is
        // asked to open, and either way PLAY starts the connection again.
        connection.started_by_play = false;

        #[cfg(not(target_family = "wasm"))]
        commands.trigger(crate::menu::ShowMenu);

        #[cfg(target_family = "wasm")]
        crate::page::request_menu();
    }
}
