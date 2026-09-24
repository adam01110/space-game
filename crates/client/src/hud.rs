use std::collections::{HashMap, HashSet};

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
type RemoteShips<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform), (With<Player>, Without<InputMarker<PlayerInput>>)>;

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

#[derive(Component)]
pub(super) struct HudMapBlip;

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
            .add_systems(Update, (update_hud, update_hud_health))
            .add_systems(Update, (update_hud_map, update_hud_visibility).chain());
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
    mut blips: Query<&mut Visibility, With<HudMapBlip>>,
) {
    *cycle = (*cycle + time.delta_secs()) % MAP_CYCLE_SECONDS;
    let playing = !local_player.is_empty();
    let blips_shown = playing && *cycle < MAP_VISIBLE_SECONDS;

    for (css_id, mut visibility) in hud.iter_mut() {
        let shown = match css_id.0.as_str() {
            "hud-map" | "hud-status" | "hud-abilities" | "hud-health" | "hud-alert" => playing,
            _ => continue,
        };

        let wanted = match shown {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        };

        if *visibility != wanted {
            *visibility = wanted;
        }
    }

    let wanted = match blips_shown {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    };

    for mut visibility in &mut blips {
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

// Places every replicated ship on the ring. The map is centred on the local ship,
// so it reads as the view from that ship rather than the arena.
pub(super) fn update_hud_map(
    mut commands: Commands,
    window: Single<&Window, With<PrimaryWindow>>,
    local: Query<&Transform, (With<Player>, With<InputMarker<PlayerInput>>)>,
    remotes: RemoteShips,
    ring: Query<(Entity, &CssID)>,
    mut blips: Query<&mut Node, With<HudMapBlip>>,
    mut entities: Local<HashMap<Entity, Entity>>,
) {
    let ring = ring
        .iter()
        .find(|(_, id)| id.0 == "hud-map-ring")
        .map(|(entity, _)| entity);
    let local = local.single().ok();
    let mut visible = HashSet::new();
    if let (Some(ring), Some(local)) = (ring, local) {
        let scale = MAP_RADIUS_PX / map_range(window.width(), window.height());
        let origin = local.translation.truncate();

        for (player, transform) in &remotes {
            let offset = (transform.translation.truncate() - origin) * scale;
            if offset.length() > MAP_RADIUS_PX {
                continue;
            }

            visible.insert(player);
            match entities.get(&player) {
                Some(&blip) => {
                    if let Ok(mut node) = blips.get_mut(blip) {
                        position_blip(&mut node, offset);
                    }
                }
                None => {
                    let mut node = Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(BLIP_SIZE_PX),
                        height: Val::Px(BLIP_SIZE_PX),
                        ..default()
                    };
                    position_blip(&mut node, offset);
                    let blip = commands
                        .spawn((
                            HudMapBlip,
                            node,
                            BackgroundColor(Color::srgb_u8(0xa6, 0x63, 0x72)),
                            UiTransform::from_rotation(Rot2::degrees(45.0)),
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(ring).add_child(blip);
                    entities.insert(player, blip);
                }
            }
        }
    }

    entities.retain(|player, blip| match visible.contains(player) {
        true => true,
        false => {
            commands.entity(*blip).despawn();
            false
        }
    });
}

fn position_blip(node: &mut Node, offset: Vec2) {
    let half = BLIP_SIZE_PX / 2.0;
    node.left = Val::Px(MAP_RADIUS_PX + offset.x - half);
    node.top = Val::Px(MAP_RADIUS_PX - offset.y - half);
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
