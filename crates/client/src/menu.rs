use bevy::prelude::*;
use bevy_extended_ui::{
    ExtendedUiPlugin,
    html::HtmlSource,
    styles::CssID,
    widgets::{Body, Button, Paragraph},
};
use lightyear::prelude::input::native::InputMarker;

use space_game_protocol::PlayerInput;

use crate::network::GuestConnection;

pub(super) struct NativeMenuPlugin;

impl Plugin for NativeMenuPlugin {
    fn build(&self, app: &mut App) {
        // The framework's index entrypoint owns menu creation.
        assert!(app.is_plugin_added::<ExtendedUiPlugin>());
        let component = &crate::menu_component::MENU_COMPONENT;
        debug_assert_eq!(component.template_name, "app-menu");
        debug_assert_eq!(component.template_file, "menu.component.html");
        debug_assert_eq!(component.styles, &["menu.css"]);
        app.add_observer(play_clicked)
            .add_systems(Update, sync_menu_status);
    }
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
    bodies: Query<(Entity, &Body)>,
    mut statuses: Query<(&CssID, &mut Paragraph)>,
    mut closed: Local<bool>,
) {
    if !connection.started_by_play || *closed {
        return;
    }
    if !players.is_empty() {
        // Retire both the framework source and its rendered tree so asset reloads
        // cannot recreate the menu after entering the game.
        for (entity, source) in &sources {
            if source.source_id == "framework-index" {
                commands.entity(entity).despawn();
            }
        }
        for (entity, body) in &bodies {
            if body.html_key.as_deref() == Some("framework-index") {
                commands.entity(entity).despawn();
            }
        }
        *closed = true;
        return;
    }

    let message = if connection.message.is_empty() {
        "Waiting for your player…"
    } else {
        &connection.message
    };

    for (id, mut status) in &mut statuses {
        if id.0 == "status" && status.text != message {
            message.clone_into(&mut status.text);
        }
    }
}
