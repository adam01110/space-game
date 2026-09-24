// The browser page owns the main menu, unlike the framework menu that the native client creates
// itself. The client therefore reports through the game shell element that the page observes, and
// receives Play clicks through the `play` export below.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bevy::prelude::*;
use lightyear::prelude::input::native::InputMarker;
use wasm_bindgen::prelude::wasm_bindgen;

use space_game_protocol::PlayerInput;

// Element in `web/index.html` that carries every signal the client sends.
const SHELL_ID: &str = "game-shell";
// Set while a local player exists: the page keeps the menu up until there is a game to show.
const READY_ATTRIBUTE: &str = "data-game-ready";
// Number of times the client asked for the menu; the page opens it on every increase.
const MENU_ATTRIBUTE: &str = "data-menu-requests";

// A page click cannot reach the running app directly, so it is handed over through these.
static PLAY_REQUESTED: AtomicBool = AtomicBool::new(false);
static MENU_REQUESTS: AtomicUsize = AtomicUsize::new(0);

pub(super) struct PagePlugin;

impl Plugin for PagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Ready>()
            .add_systems(Update, signal_game_ready);
    }
}

// The readiness the page was last told about, so a transition writes the element once.
#[derive(Resource, Default)]
struct Ready(bool);

// Called by the page on every Play click. The module's own startup only covers the first session
// because the page boots it from that same button, so later sessions arrive here instead.
#[wasm_bindgen]
pub fn play() {
    PLAY_REQUESTED.store(true, Ordering::Relaxed);
}

pub(super) fn take_play_request() -> bool {
    PLAY_REQUESTED.swap(false, Ordering::Relaxed)
}

// Ask the page to open its menu again.
pub(super) fn request_menu() {
    let requests = MENU_REQUESTS.fetch_add(1, Ordering::Relaxed) + 1;
    let _ = set_attribute(MENU_ATTRIBUTE, &requests.to_string());
}

// Readiness follows the local player rather than the client link: the page reveals the game once
// there is a ship to fly, and hides it again when the session that owned the ship ended.
fn signal_game_ready(players: Query<(), With<InputMarker<PlayerInput>>>, mut ready: ResMut<Ready>) {
    let ready_now = !players.is_empty();
    if ready_now == ready.0 {
        return;
    }

    let value = if ready_now { "true" } else { "false" };
    if set_attribute(READY_ATTRIBUTE, value) {
        ready.0 = ready_now;
    }
}

// Reports whether the element took the attribute.
fn set_attribute(name: &str, value: &str) -> bool {
    shell().is_some_and(|shell| shell.set_attribute(name, value).is_ok())
}

fn shell() -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(SHELL_ID)
}
