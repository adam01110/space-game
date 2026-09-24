use bevy::prelude::*;
use lightyear::prelude::{input::native::InputMarker, Controlled};

use space_game_protocol::{Player, PlayerHealth, PlayerInput};

use crate::menu::ShowMenu;
use crate::network::{death, GuestConnection};

// The menu owns its own rebuilding; this only records that it was asked to come back.
#[derive(Resource, Default)]
struct MenuRequests(usize);

fn count_menu_requests(_event: On<ShowMenu>, mut requests: ResMut<MenuRequests>) {
    requests.0 += 1;
}

fn client() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
    app.insert_resource(GuestConnection::default());
    app.insert_resource(MenuRequests::default());
    app.add_observer(count_menu_requests);
    death::install(&mut app);
    app
}

fn requested_menu(app: &App) -> usize {
    app.world().resource::<MenuRequests>().0
}

#[test]
fn a_destroyed_player_ends_the_session_and_asks_for_the_menu_once() {
    let mut app = client();
    app.world_mut().spawn((
        Player,
        Controlled,
        InputMarker::<PlayerInput>::default(),
        PlayerHealth(0),
    ));
    app.update();

    assert_eq!(requested_menu(&app), 1);
    let connection = app.world().resource::<GuestConnection>();
    assert!(!connection.started_by_play, "PLAY starts the next session");
    assert!(connection.client.is_none() && connection.pending.is_none());

    // A destroyed player that outlives the cleanup does not end the session twice.
    app.update();
    assert_eq!(requested_menu(&app), 1, "one death is handled once");
}

#[test]
fn a_destroyed_player_without_local_input_does_not_end_the_session() {
    let mut app = client();
    app.world_mut().spawn((Player, Controlled, PlayerHealth(0)));
    app.update();

    assert_eq!(requested_menu(&app), 0);
}

#[test]
fn a_living_player_keeps_its_session() {
    let mut app = client();
    app.world_mut().spawn((
        Player,
        Controlled,
        InputMarker::<PlayerInput>::default(),
        PlayerHealth(1),
    ));
    app.update();

    assert_eq!(requested_menu(&app), 0);
}
