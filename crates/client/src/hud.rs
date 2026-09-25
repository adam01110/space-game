use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_extended_ui::{ExtendedUiPlugin, styles::CssID, widgets::Paragraph};
use lightyear::prelude::client::Client;
use lightyear::prelude::input::native::InputMarker;
use lightyear::prelude::{PingManager, Predicted};

use space_game_protocol::{
    ArenaBoundary, Player, PlayerBlasters, PlayerBoost, PlayerHealth, PlayerInput, PlayerPhaseBeam,
};

// Charged abilities are read from the local player's predicted components, so the readout
// matches the ship the client flies.
type LocalCharges<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerBoost,
        &'static PlayerBlasters,
        &'static PlayerPhaseBeam,
    ),
    With<InputMarker<PlayerInput>>,
>;

// Keep the player marker inside the arena ring, including its own width.
const MAP_RADIUS_PX: f32 = 35.0;
const MAP_CENTER_PX: f32 = 39.0;
const MARKER_HALF_PX: f32 = 4.0;

// Full width of the hull bar; the readout scales the filled box by the ship's health.
pub(super) const HEALTH_BAR_WIDTH_PX: f32 = 140.0;

pub(super) struct NativeHudPlugin;

impl Plugin for NativeHudPlugin {
    fn build(&self, app: &mut App) {
        // Extended UI owns HUD creation: the framework entrypoint on native, an HTML asset on wasm.
        assert!(app.is_plugin_added::<ExtendedUiPlugin>());
        #[cfg(not(target_family = "wasm"))]
        {
            let component = &crate::hud_component::HUD_COMPONENT;
            debug_assert_eq!(component.template_name, "app-hud");
            debug_assert_eq!(component.template_file, "hud.component.html");
            debug_assert_eq!(component.styles, &["hud.css"]);
        }

        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Update, (update_hud, update_hud_health, update_hud_map))
            .add_systems(Update, update_hud_visibility);
    }
}

fn update_hud(
    players: Query<(), With<Player>>,
    charges: LocalCharges,
    pings: Query<&PingManager, With<Client>>,
    diagnostics: Res<DiagnosticsStore>,
    mut hud: Query<(&CssID, &mut Paragraph)>,
) {
    write_line(&mut hud, "hud-fps", &fps_text(&diagnostics));
    write_line(&mut hud, "hud-ping", &ping_text(&pings));
    write_line(
        &mut hud,
        "hud-players",
        &format!("PLAYERS: {}", players.iter().count()),
    );

    // Before the local player exists the charge rows keep their last value; the
    // menu covers the overlay until then.
    let Ok((boost, blasters, beam)) = charges.single() else {
        return;
    };

    write_line(&mut hud, "hud-boost", &charge_text(boost.0.units()));
    write_line(&mut hud, "hud-blaster", &charge_text(blasters.0.units()));
    write_line(&mut hud, "hud-beam", &charge_text(beam.0.units()));
}

// Hide both overlay roots while the menu owns the screen. Hiding the HUD root also hides
// its readouts; the frame is a separate sibling in the framework entrypoint.
pub(super) fn update_hud_visibility(
    local_player: Query<(), (With<Player>, With<InputMarker<PlayerInput>>)>,
    mut hud: Query<(&CssID, &mut Visibility)>,
) {
    let playing = !local_player.is_empty();

    for (id, mut visibility) in &mut hud {
        if matches!(
            id.0.as_str(),
            "hud"
                | "frame-layer"
                | "hud-map"
                | "hud-status"
                | "hud-abilities"
                | "hud-health"
                | "hud-alert"
        ) {
            set_visibility(&mut visibility, playing);
        }
    }
}

fn set_visibility(visibility: &mut Visibility, shown: bool) {
    let wanted = match shown {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    };

    if *visibility != wanted {
        *visibility = wanted;
    }
}

// The hull readout follows the local ship's predicted health, so it reacts on the tick the client
// sees the damage; the menu covers the overlay before that ship exists.
pub(super) fn update_hud_health(
    health: Query<&PlayerHealth, (With<Player>, With<InputMarker<PlayerInput>>)>,
    mut hud: Query<(&CssID, &mut Paragraph)>,
    mut nodes: Query<(&CssID, &mut Node)>,
) {
    let Ok(health) = health.single() else {
        return;
    };

    write_line(&mut hud, "hud-health-value", &health.0.to_string());

    let filled = f32::from(health.0.min(PlayerHealth::FULL)) / f32::from(PlayerHealth::FULL)
        * HEALTH_BAR_WIDTH_PX;

    for (css_id, mut node) in nodes.iter_mut() {
        if css_id.0 == "hud-health-fill" && node.width != Val::Px(filled) {
            node.width = Val::Px(filled);
        }
    }
}

// The ring represents the entire arena, centred on the world's origin. Only the local
// predicted ship is marked; changing arena size rescales its position on the ring.
pub(super) fn update_hud_map(
    local: Query<&Transform, (With<Player>, With<InputMarker<PlayerInput>>)>,
    arenas: Query<&ArenaBoundary, With<Predicted>>,
    mut markers: Query<(&CssID, &mut Node, &mut Visibility)>,
) {
    let position = local
        .single()
        .ok()
        .zip(arenas.single().ok())
        .and_then(|(player, arena)| {
            (arena.radius > 0.0).then(|| {
                (player.translation.truncate() / arena.radius).clamp_length_max(1.0) * MAP_RADIUS_PX
            })
        });

    for (id, mut node, mut visibility) in &mut markers {
        match id.0 != "hud-map-player" {
            true => continue,
            false => {
                set_visibility(&mut visibility, position.is_some());

                if let Some(offset) = position {
                    node.left = Val::Px(MAP_CENTER_PX + offset.x - MARKER_HALF_PX);
                    node.top = Val::Px(MAP_CENTER_PX - offset.y - MARKER_HALF_PX);
                }
            }
        }
    }
}

// Writes only when the rendered text differs, so unchanged values never re-layout.
fn write_line(hud: &mut Query<(&CssID, &mut Paragraph)>, id: &str, text: &str) {
    for (css_id, mut paragraph) in hud.iter_mut() {
        if css_id.0 == id && paragraph.text != text {
            text.clone_into(&mut paragraph.text);
        }
    }
}

fn fps_text(diagnostics: &DiagnosticsStore) -> String {
    diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(Diagnostic::smoothed)
        .map_or_else(|| String::from("FPS: --"), |fps| format!("FPS: {fps:.0}"))
}

fn ping_text(pings: &Query<&PingManager, With<Client>>) -> String {
    pings.single().map_or_else(
        |_| String::from("Ping: -- Ms"),
        |ping| format!("Ping: {} ms", ping.rtt().as_millis()),
    )
}

fn charge_text(units: u8) -> String {
    format!("{units:03}")
}
