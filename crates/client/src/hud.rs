use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_extended_ui::{ExtendedUiPlugin, styles::CssID, widgets::Paragraph};
use lightyear::prelude::PingManager;
use lightyear::prelude::client::Client;
use lightyear::prelude::input::native::InputMarker;

use space_game_protocol::{
    Player, PlayerBlasters, PlayerBoost, PlayerHealth, PlayerInput, PlayerPhaseBeam,
};

// Charged abilities are read from the local player's predicted components, so the
// readout matches the ship the client actually flies.
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

// The minimap covers the view the client renders plus a margin, so a blip at the
// ring is a ship just off screen. The ring's inner radius is that range.
const MAP_RADIUS_PX: f32 = 38.0;
const MAP_RANGE_MARGIN: f32 = 1.1;
const BLIP_SIZE_PX: f32 = 8.0;

// Full width of the hull bar; the readout scales the filled box by the ship's health.
pub(super) const HEALTH_BAR_WIDTH_PX: f32 = 140.0;
const MAP_BLIP_IDS: [&str; 8] = [
    "hud-map-blip-0",
    "hud-map-blip-1",
    "hud-map-blip-2",
    "hud-map-blip-3",
    "hud-map-blip-4",
    "hud-map-blip-5",
    "hud-map-blip-6",
    "hud-map-blip-7",
];

// The blips fade out for twice as long as they show, so the map never reads as a
// static icon sitting in the corner.
const MAP_VISIBLE_SECONDS: f32 = 0.5;
const MAP_HIDDEN_SECONDS: f32 = 1.0;
const MAP_CYCLE_SECONDS: f32 = MAP_VISIBLE_SECONDS + MAP_HIDDEN_SECONDS;

pub(super) struct NativeHudPlugin;

impl Plugin for NativeHudPlugin {
    fn build(&self, app: &mut App) {
        // The framework's index entrypoint owns HUD creation.
        assert!(app.is_plugin_added::<ExtendedUiPlugin>());
        let component = &crate::hud_component::HUD_COMPONENT;
        debug_assert_eq!(component.template_name, "app-hud");
        debug_assert_eq!(component.template_file, "hud.component.html");
        debug_assert_eq!(component.styles, &["hud.css"]);
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(
                Update,
                (
                    update_hud,
                    update_hud_health,
                    update_hud_map,
                    update_hud_visibility,
                ),
            );
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

// The readouts belong to a running match: the menu owns the screen until a player
// exists, and the blips then blink on their own cycle.
fn update_hud_visibility(
    local_player: Query<(), (With<Player>, With<InputMarker<PlayerInput>>)>,
    time: Res<Time>,
    mut cycle: Local<f32>,
    mut hud: Query<(&CssID, &mut Visibility)>,
) {
    *cycle = (*cycle + time.delta_secs()) % MAP_CYCLE_SECONDS;
    let playing = !local_player.is_empty();
    let blips_shown = playing && *cycle < MAP_VISIBLE_SECONDS;

    for (css_id, mut visibility) in hud.iter_mut() {
        let shown = if is_blip(&css_id.0) {
            blips_shown
        } else if matches!(
            css_id.0.as_str(),
            "hud-map" | "hud-status" | "hud-abilities" | "hud-health" | "hud-alert"
        ) {
            playing
        } else {
            continue;
        };
        let wanted = match shown {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

// The hull readout follows the local ship's predicted health, so it reacts on the tick the
// client sees the damage. Before that ship exists the menu covers the overlay.
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

// Only the ships blink; the ring, the grid and the label stay put.
fn is_blip(id: &str) -> bool {
    MAP_BLIP_IDS.contains(&id)
}

// Places every replicated ship on the ring. The map is centred on the local ship,
// so it reads as the view from that ship rather than the arena.
fn update_hud_map(
    window: Single<&Window, With<PrimaryWindow>>,
    local: Query<&Transform, (With<Player>, With<InputMarker<PlayerInput>>)>,
    remotes: Query<&Transform, (With<Player>, Without<InputMarker<PlayerInput>>)>,
    mut nodes: Query<(&CssID, &mut Node)>,
) {
    let Ok(local) = local.single() else {
        return;
    };
    let range = map_range(window.width(), window.height());
    let scale = MAP_RADIUS_PX / range;
    let origin = local.translation.truncate();

    // The local ship sits at the centre of its own map, so it gets no blip.
    // Interest management only replicates nearby ships, so the pool takes whatever
    // the client knows about and hides the rest. Ships past the map's range are left
    // off entirely instead of piling up on the ring.
    let mut free = MAP_BLIP_IDS.iter();
    for transform in &remotes {
        let offset = (transform.translation.truncate() - origin) * scale;
        if offset.length() > MAP_RADIUS_PX {
            continue;
        }
        let Some(id) = free.next() else {
            break;
        };
        place(&mut nodes, id, offset, BLIP_SIZE_PX);
    }
    for id in free {
        hide(&mut nodes, id);
    }
}

// The rendered view is one world unit per window pixel: the gameplay camera draws
// `PIXEL_SIZE` world units per canvas pixel and the canvas is the window divided by
// the same factor.
fn map_range(width: f32, height: f32) -> f32 {
    Vec2::new(width, height).length() / 2.0 * MAP_RANGE_MARGIN
}

// Writes only when the rendered text differs, so unchanged values never re-layout.
fn write_line(hud: &mut Query<(&CssID, &mut Paragraph)>, id: &str, text: &str) {
    for (css_id, mut paragraph) in hud.iter_mut() {
        if css_id.0 == id && paragraph.text != text {
            text.clone_into(&mut paragraph.text);
        }
    }
}

// Maps a world offset onto the ring and writes the blip's box.
fn place(nodes: &mut Query<(&CssID, &mut Node)>, id: &str, offset: Vec2, size: f32) {
    let half = size / 2.0;
    for (css_id, mut node) in nodes.iter_mut() {
        if css_id.0 == id {
            node.left = Val::Px(MAP_RADIUS_PX + offset.x - half);
            node.top = Val::Px(MAP_RADIUS_PX - offset.y - half);
            node.display = Display::Flex;
        }
    }
}

fn hide(nodes: &mut Query<(&CssID, &mut Node)>, id: &str) {
    for (css_id, mut node) in nodes.iter_mut() {
        if css_id.0 == id {
            node.display = Display::None;
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
