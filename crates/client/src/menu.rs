use bevy::prelude::*;
use bevy_extended_ui::{
    ExtendedUiPlugin,
    html::HtmlSource,
    io::HtmlAsset,
    styles::CssID,
    widgets::{Button, Paragraph},
};
use lightyear::prelude::input::native::InputMarker;

use space_game_protocol::PlayerInput;

use crate::network::GuestConnection;

// Registry key of the framework entry point. The menu owns the key because it also removes
// the source once the game starts.
const FRAMEWORK_INDEX: &str = "framework-index";

pub(super) struct NativeMenuPlugin;

impl Plugin for NativeMenuPlugin {
    fn build(&self, app: &mut App) {
        // The framework's index entrypoint owns menu creation.
        assert!(app.is_plugin_added::<ExtendedUiPlugin>());
        let component = &crate::menu_component::MENU_COMPONENT;
        debug_assert_eq!(component.template_name, "app-menu");
        debug_assert_eq!(component.template_file, "menu.component.html");
        debug_assert_eq!(component.styles, &["menu.css"]);
        app.init_resource::<MenuState>()
            .add_observer(play_clicked)
            .add_observer(show_menu)
            .add_systems(Update, sync_menu_status);
    }
}

// Whether the framework menu is currently on screen. The menu is retired when the local
// player joins and rebuilt when the session ends.
#[derive(Resource, Default)]
pub(super) struct MenuState {
    retired: bool,
}

// Raised when a session ended and the menu should come back.
#[derive(Event)]
pub(super) struct ShowMenu;

// Rebuild the menu after the session that replaced it ended. The framework treats a new
// source as a fresh entrypoint, so the same key and asset produce a new tree.
fn show_menu(
    _event: On<ShowMenu>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<MenuState>,
) {
    let handle: Handle<HtmlAsset> = asset_server.load("index.html");
    commands.spawn(HtmlSource {
        handle,
        source_id: FRAMEWORK_INDEX.to_owned(),
        controller: None,
    });
    state.retired = false;
}

fn play_clicked(
    event: On<Pointer<Click>>,
    buttons: Query<&CssID, With<Button>>,
    mut connection: ResMut<GuestConnection>,
    time: Res<Time<Real>>,
) {
    if connection.started_by_play || !buttons.get(event.entity).is_ok_and(|id| id.0 == "play") {
        return;
    }
    connection.started_by_play = true;
    connection.request(time.elapsed());
}

fn sync_menu_status(
    mut commands: Commands,
    connection: Res<GuestConnection>,
    players: Query<(), With<InputMarker<PlayerInput>>>,
    sources: Query<(Entity, &HtmlSource)>,
    slots: Query<(Entity, &CssID)>,
    mut statuses: Query<(&CssID, &mut Paragraph)>,
    mut state: ResMut<MenuState>,
) {
    if !connection.started_by_play || state.retired {
        return;
    }
    if !players.is_empty() {
        // Retire the framework source and the menu's shell slot so asset reloads cannot
        // recreate the menu after entering the game. The HUD shares the framework index
        // and has to survive the menu.
        retire_sources(&mut commands, &sources);
        retire_slot(&mut commands, &slots);
        state.retired = true;
        return;
    }

    update_status(&mut statuses, status_message(&connection));
}

fn retire_sources(commands: &mut Commands, sources: &Query<(Entity, &HtmlSource)>) {
    for (entity, source) in sources {
        if source.source_id == FRAMEWORK_INDEX {
            commands.entity(entity).despawn();
        }
    }
}

fn retire_slot(commands: &mut Commands, slots: &Query<(Entity, &CssID)>) {
    for (entity, id) in slots {
        if id.0 == "menu-slot" {
            commands.entity(entity).despawn();
        }
    }
}

fn status_message(connection: &GuestConnection) -> &str {
    match connection.message.is_empty() {
        true => "Waiting for your player…",
        false => &connection.message,
    }
}

fn update_status(statuses: &mut Query<(&CssID, &mut Paragraph)>, message: &str) {
    for (id, mut status) in statuses {
        if id.0 == "status" && status.text != message {
            message.clone_into(&mut status.text);
        }
    }
}
