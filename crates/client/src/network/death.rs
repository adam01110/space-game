use bevy::prelude::*;
use lightyear::prelude::{Controlled, input::native::InputMarker};

use space_game_protocol::{Player, PlayerHealth, PlayerInput};

use super::{GuestConnection, connection::retire_session, update_connection};

type LocalPlayer = (
    With<Player>,
    With<Controlled>,
    With<InputMarker<PlayerInput>>,
);

pub(crate) fn install(app: &mut App) {
    app.add_systems(Update, end_dead_session.before(update_connection));
}

// Health reaches zero in the authoritative simulation, so the client only reacts to it.
// Retiring the session leaves the server to despawn the destroyed player.
fn end_dead_session(
    mut commands: Commands,
    mut connection: ResMut<GuestConnection>,
    players: Query<&PlayerHealth, LocalPlayer>,
    mut ended: Local<bool>,
    #[cfg(target_family = "wasm")] time: Res<Time<Real>>,
) {
    let destroyed = players.iter().any(|health| health.0 == 0);

    if !destroyed {
        *ended = false;
        return;
    }
    if *ended {
        return;
    }
    *ended = true;

    warn!("Player destroyed; returning to the menu");
    retire_session(&mut commands, &mut connection);

    #[cfg(not(target_family = "wasm"))]
    {
        // The menu is the native entry point: PLAY starts the next session.
        connection.started_by_play = false;
        commands.trigger(crate::menu::ShowMenu);
    }
    #[cfg(target_family = "wasm")]
    connection.request(time.elapsed());
}
