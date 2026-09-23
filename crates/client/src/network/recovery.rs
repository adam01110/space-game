use std::time::Duration;

use bevy::{prelude::*, time::TimeSystems};

use super::{GuestConnection, connection::retire_session};

// Bevy clamps virtual time at 250 ms by default. Retire before repeatedly clamped
// frames can leave the input timeline permanently behind wall time.
pub(crate) const MAX_FRAME_GAP: Duration = Duration::from_millis(250);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Recovery {
    None,
    Suspend,
    Reconnect,
}

#[derive(Resource, Default)]
pub(crate) struct Suspension {
    hidden_since: Option<Duration>,
    suspended: bool,
}

impl Suspension {
    pub(crate) fn observe(&mut self, now: Duration, gap: Duration, hidden: bool) -> Recovery {
        match hidden {
            true => {
                let since = *self.hidden_since.get_or_insert(now);

                match !self.suspended
                    && (gap >= MAX_FRAME_GAP || now.saturating_sub(since) >= MAX_FRAME_GAP)
                {
                    true => {
                        self.suspended = true;
                        return Recovery::Suspend;
                    }
                    false => return Recovery::None,
                }
            }
            false => {
                let long_hidden = self
                    .hidden_since
                    .take()
                    .is_some_and(|since| now.saturating_sub(since) >= MAX_FRAME_GAP);

                match std::mem::take(&mut self.suspended) || long_hidden || gap >= MAX_FRAME_GAP {
                    true => Recovery::Reconnect,
                    false => Recovery::None,
                }
            }
        }
    }

    pub(crate) const fn is_suspended(&self) -> bool {
        self.suspended
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Suspension>()
        .add_systems(First, recover_suspended_session.after(TimeSystems));
}

// Document visibility is deliberately not window focus: an unfocused native
// window (or visible browser window) can continue rendering normally.
#[cfg(target_family = "wasm")]
fn browser_hidden() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.hidden())
}

#[cfg(not(target_family = "wasm"))]
const fn browser_hidden() -> bool {
    false
}

fn recover_suspended_session(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut suspension: ResMut<Suspension>,
    connection: Option<ResMut<GuestConnection>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    // An unfocused native window normally continues at 60 Hz. Only suspend it
    // if its compositor actually throttles updates; avoid reconnecting every
    // throttled frame while it remains unfocused.
    let throttled_unfocused = windows.single().is_ok_and(|window| !window.focused)
        && (time.delta() >= MAX_FRAME_GAP || suspension.is_suspended());
    let recovery = suspension.observe(
        time.elapsed(),
        time.delta(),
        browser_hidden() || throttled_unfocused,
    );

    let Some(mut connection) = connection else {
        return;
    };
    #[cfg(not(target_family = "wasm"))]
    if !connection.started_by_play {
        return;
    }
    if recovery == Recovery::None {
        return;
    }

    warn!(?recovery, gap = ?time.delta(), "Retiring stale network session after suspension");

    // First's deferred commands (including Lightyear receiver-removal cleanup)
    // are flushed before any PreUpdate packet receive / rollback systems run.
    retire_session(&mut commands, &mut connection);
    match recovery {
        Recovery::Reconnect => connection.request(time.elapsed()),
        Recovery::Suspend => {
            connection.can_retry = false;
            "Paused while client is inactive".clone_into(&mut connection.message);
        }
        Recovery::None => {}
    }
}
