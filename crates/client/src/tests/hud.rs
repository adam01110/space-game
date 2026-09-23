use bevy::prelude::*;
use bevy_extended_ui::{styles::CssID, widgets::Paragraph};
use lightyear::prelude::input::native::InputMarker;

use space_game_protocol::{Player, PlayerHealth, PlayerInput};

use crate::hud::{HEALTH_BAR_WIDTH_PX, update_hud_health};

// The framework builds the HUD from the stylesheet, so the readout only has to find the
// nodes the stylesheet names.
fn readout() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, update_hud_health);
    app.world_mut()
        .spawn((CssID("hud-health-value".to_owned()), Paragraph::default()));
    app.world_mut()
        .spawn((CssID("hud-health-fill".to_owned()), Node::default()));
    app
}

fn text(app: &mut App, id: &str) -> String {
    let mut query = app.world_mut().query::<(&CssID, &Paragraph)>();

    query
        .iter(app.world())
        .find(|(css_id, _)| css_id.0 == id)
        .map(|(_, paragraph)| paragraph.text.clone())
        .expect("hud text node")
}

fn width(app: &mut App, id: &str) -> Val {
    let mut query = app.world_mut().query::<(&CssID, &Node)>();

    query
        .iter(app.world())
        .find(|(css_id, _)| css_id.0 == id)
        .map(|(_, node)| node.width)
        .expect("hud node")
}

#[test]
fn the_hull_readout_follows_the_local_ship() {
    let mut app = readout();
    app.world_mut().spawn((
        Player,
        InputMarker::<PlayerInput>::default(),
        PlayerHealth(PlayerHealth::FULL / 2),
    ));
    app.update();

    assert_eq!(text(&mut app, "hud-health-value"), "50");
    assert_eq!(
        width(&mut app, "hud-health-fill"),
        Val::Px(HEALTH_BAR_WIDTH_PX / 2.0)
    );
}

#[test]
fn the_hull_bar_never_exceeds_its_track() {
    let mut app = readout();
    app.world_mut().spawn((
        Player,
        InputMarker::<PlayerInput>::default(),
        PlayerHealth(u8::MAX),
    ));
    app.update();

    assert_eq!(text(&mut app, "hud-health-value"), u8::MAX.to_string());
    assert_eq!(
        width(&mut app, "hud-health-fill"),
        Val::Px(HEALTH_BAR_WIDTH_PX)
    );
}

#[test]
fn a_remote_ship_does_not_drive_the_hull_readout() {
    let mut app = readout();
    app.world_mut().spawn((Player, PlayerHealth(30)));
    app.update();

    assert_eq!(text(&mut app, "hud-health-value"), "");
}
