use std::net::{Ipv4Addr, SocketAddr};

use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::{
    netcode::{NetcodeClient, client_plugin::NetcodeConfig, generate_key},
    prelude::{
        client::{input::InputSystems, *},
        input::native::{ActionState, InputMarker},
        *,
    },
};
use project_game::{ClientSimulationPlugin, GamePlugin};
use project_protocol::{
    PRIVATE_KEY, PROTOCOL_ID, Player, PlayerInput, PlayerPosition, ProtocolPlugin, SERVER_PORT,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ClientPlugins::default(),
            ProtocolPlugin,
            GamePlugin,
            ClientSimulationPlugin,
        ))
        .add_systems(
            Startup,
            (setup_camera, spawn_client, connect_client).chain(),
        )
        .add_systems(
            FixedPreUpdate,
            buffer_player_input.in_set(InputSystems::WriteClientInputs),
        )
        .add_observer(prepare_controlled_player)
        .add_observer(add_predicted_player_visual)
        .add_observer(add_interpolated_player_visual)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_client(mut commands: Commands) {
    let client_id = u64::from_le_bytes(
        generate_key()[..size_of::<u64>()]
            .try_into()
            .expect("client ID source must contain eight bytes"),
    );
    let server_address = SocketAddr::from((Ipv4Addr::LOCALHOST, SERVER_PORT));
    let authentication = Authentication::Manual {
        server_addr: server_address,
        client_id,
        private_key: PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };

    commands.insert_resource(PredictionManager::default());
    commands.spawn((
        Name::new(format!("Client {client_id}")),
        Client,
        ReplicationReceiver,
        Link::default(),
        LocalAddr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))),
        PeerAddr(server_address),
        PingManager::default(),
        NetcodeClient::new(authentication, NetcodeConfig::default())
            .expect("failed to configure Netcode client"),
        WebTransportClientIo {
            certificate_digest: String::new(),
            target: None,
        },
    ));
}

fn connect_client(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}

fn prepare_controlled_player(
    trigger: On<Add, Controlled>,
    players: Query<(), (With<Player>, Without<InputMarker<PlayerInput>>)>,
    mut commands: Commands,
) {
    if players.contains(trigger.entity) {
        commands
            .entity(trigger.entity)
            .insert(InputMarker::<PlayerInput>::default());
    }
}

fn add_predicted_player_visual(trigger: On<Add, (Player, Predicted)>, mut commands: Commands) {
    add_player_visual(&mut commands, trigger.entity, Color::srgb(0.35, 0.75, 1.0));
}

fn add_interpolated_player_visual(
    trigger: On<Add, (Player, Interpolated)>,
    mut commands: Commands,
) {
    add_player_visual(&mut commands, trigger.entity, Color::srgb(1.0, 0.4, 0.35));
}

fn add_player_visual(commands: &mut Commands, entity: Entity, color: Color) {
    commands.entity(entity).insert((
        Sprite::from_color(color, Vec2::new(28.0, 44.0)),
        Transform::default(),
    ));
}

fn buffer_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut players: Query<
        (&PlayerPosition, &mut ActionState<PlayerInput>),
        With<InputMarker<PlayerInput>>,
    >,
) {
    let Ok((position, mut action_state)) = players.single_mut() else {
        return;
    };

    let horizontal = axis(&keyboard, KeyCode::KeyA, KeyCode::KeyD);
    let vertical = axis(&keyboard, KeyCode::KeyS, KeyCode::KeyW);
    let mut aim = action_state.0.aim;

    if let (Ok(window), Ok((camera, camera_transform))) = (windows.single(), cameras.single()) {
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok(cursor_world) = camera.viewport_to_world_2d(camera_transform, cursor_position)
            {
                if let Some(direction) = (cursor_world - position.0).try_normalize() {
                    aim = direction;
                }
            }
        }
    }

    action_state.0 = PlayerInput {
        movement: Vec2::new(horizontal, vertical),
        aim,
    };
}

fn axis(keyboard: &ButtonInput<KeyCode>, negative: KeyCode, positive: KeyCode) -> f32 {
    let positive = if keyboard.pressed(positive) { 1.0 } else { 0.0 };
    let negative = if keyboard.pressed(negative) { 1.0 } else { 0.0 };

    positive - negative
}
