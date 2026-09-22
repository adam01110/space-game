//! Real Lightyear packets over a deterministic test wire, without sockets or GPU.
use std::{collections::VecDeque, time::Duration};

use avian2d::prelude::Position;
use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use lightyear::{
    link::{SendPayload, recv_payload_from_bytes},
    prelude::{
        client::*,
        input::native::{ActionState, InputMarker},
        server::*,
        *,
    },
};
use space_game_game::{ClientSimulationPlugin, GamePlugin, PlayerBundle, ServerSimulationPlugin};
use space_game_protocol::{BlasterShot, Player, PlayerIdentity, PlayerInput, ProtocolPlugin};
use space_game_server::ServerAbilitiesPlugin;

use crate::network::policy;

const FRAME: Duration = Duration::from_millis(10);
const TICK: Duration = Duration::from_nanos(16_666_667);

#[derive(Default)]
struct Wire {
    packets: VecDeque<(Duration, SendPayload)>,
    sequence: u64,
    delay: Duration,
    loss: bool,
}

impl Wire {
    fn collect(&mut self, app: &mut App, endpoint: Entity, now: Duration) {
        let mut link = app.world_mut().get_mut::<Link>(endpoint).expect("link");
        for packet in link.send.drain() {
            self.sequence += 1;
            if !self.loss || !self.sequence.is_multiple_of(10) {
                self.packets.push_back((now + self.delay, packet));
            }
        }
    }

    fn deliver(&mut self, app: &mut App, endpoint: Entity, now: Duration) {
        let mut link = app.world_mut().get_mut::<Link>(endpoint).expect("link");
        while self.packets.front().is_some_and(|(due, _)| *due <= now) {
            let (_, packet) = self.packets.pop_front().expect("due packet");
            link.recv.push_raw(recv_payload_from_bytes(packet));
        }
    }
}

#[derive(Resource, Default)]
struct ScriptedInput(PlayerInput);

fn write_input(
    script: Res<ScriptedInput>,
    mut players: Query<&mut ActionState<PlayerInput>, With<InputMarker<PlayerInput>>>,
) {
    for mut state in &mut players {
        state.0.clone_from(&script.0);
    }
}

fn mark_controlled(
    trigger: On<Add, (Player, Controlled)>,
    players: Query<(), (With<Player>, With<Controlled>)>,
    mut commands: Commands,
) {
    if players.contains(trigger.entity) {
        commands
            .entity(trigger.entity)
            .insert(InputMarker::<PlayerInput>::default());
    }
}

struct Harness {
    server: App,
    client: App,
    server_link: Entity,
    client_link: Entity,
    player: Entity,
    upstream: Wire,
    downstream: Wire,
    now: Duration,
}

impl Harness {
    fn new(loss: bool) -> Self {
        let mut server = App::new();
        server.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            TransformPlugin,
            ServerPlugins {
                tick_duration: TICK,
            },
            ProtocolPlugin,
            GamePlugin,
            ServerSimulationPlugin,
            ServerAbilitiesPlugin,
        ));
        let mut client = App::new();
        client.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            TransformPlugin,
            ClientPlugins {
                tick_duration: TICK,
            },
            ProtocolPlugin,
            GamePlugin,
            ClientSimulationPlugin,
        ));
        client.insert_resource(policy::timeline_config());
        client.insert_resource(policy::prediction_manager());
        client.init_resource::<ScriptedInput>();
        client.add_observer(mark_controlled);
        client.add_systems(
            FixedPreUpdate,
            write_input.in_set(lightyear::prelude::client::input::InputSystems::WriteClientInputs),
        );
        for app in [&mut server, &mut client] {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
            app.insert_resource(ReplicationMetadata::new(TICK));
            app.finish();
            app.cleanup();
            app.update();
        }
        let root = server
            .world_mut()
            .spawn((Server::default(), Linked, Started))
            .id();
        let server_link = server
            .world_mut()
            .spawn((
                Link::default(),
                LinkOf { server: root },
                ClientOf,
                LocalId(PeerId::Server),
                RemoteId(PeerId::Local(1)),
                ReplicationSender,
                PingManager::default(),
                Linked,
            ))
            .id();
        server.world_mut().entity_mut(server_link).insert(Connected);
        let client_link = client
            .world_mut()
            .spawn((
                Link::default(),
                Client,
                LocalId(PeerId::Local(1)),
                RemoteId(PeerId::Server),
                ReplicationReceiver,
                PingManager::default(),
                Linked,
            ))
            .id();
        client.world_mut().entity_mut(client_link).insert(Connected);
        let player = server
            .world_mut()
            .spawn((
                PlayerBundle::new(Vec2::ZERO),
                PlayerIdentity(7),
                Replicate::to_clients(NetworkTarget::All),
                PredictionTarget::to_clients(NetworkTarget::All),
                ControlledBy {
                    owner: server_link,
                    lifetime: default(),
                },
            ))
            .id();
        Self {
            server,
            client,
            server_link,
            client_link,
            player,
            upstream: Wire { loss, ..default() },
            downstream: Wire {
                delay: Duration::from_millis(400),
                loss,
                ..default()
            },
            now: Duration::ZERO,
        }
    }

    fn step(&mut self) {
        self.now += FRAME;
        self.upstream
            .deliver(&mut self.server, self.server_link, self.now);
        self.downstream
            .deliver(&mut self.client, self.client_link, self.now);
        self.server.update();
        self.client.update();
        self.upstream
            .collect(&mut self.client, self.client_link, self.now);
        self.downstream
            .collect(&mut self.server, self.server_link, self.now);
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }

    fn controlled_player(&mut self) -> Entity {
        self.client
            .world_mut()
            .query_filtered::<Entity, (With<Player>, With<Controlled>, With<Predicted>)>()
            .single(self.client.world())
            .expect("controlled predicted player")
    }
}

#[test]
fn delayed_lossy_wire_converges_and_matches_projectiles() {
    for (loss, abrupt) in [(false, false), (true, false), (true, true)] {
        let mut harness = Harness::new(loss);
        if abrupt {
            harness.downstream.delay = Duration::ZERO;
        }
        harness.frames(600);
        harness.downstream.delay = Duration::from_millis(400);
        assert!(
            harness
                .client
                .world()
                .resource::<LocalTimelineSync>()
                .is_synced()
        );
        let predicted_player = harness.controlled_player();
        harness
            .client
            .world_mut()
            .resource_mut::<ScriptedInput>()
            .0
            .movement = Vec2::X;
        harness.frames(100);
        harness
            .client
            .world_mut()
            .resource_mut::<ScriptedInput>()
            .0
            .movement = Vec2::ZERO;
        harness.frames(300);
        let authoritative = harness
            .server
            .world()
            .get::<Position>(harness.player)
            .expect("server pose")
            .0;
        let predicted = harness
            .client
            .world()
            .get::<Position>(predicted_player)
            .expect("client pose")
            .0;
        assert!(authoritative.x > 100.0, "input never reached server");
        assert!(
            authoritative.distance(predicted) < 1.0,
            "loss={loss}: {authoritative:?} != {predicted:?}"
        );
        harness
            .client
            .world_mut()
            .resource_mut::<ScriptedInput>()
            .0
            .blaster_clicks = 1;
        harness.frames(4);
        let local_shot = harness
            .client
            .world_mut()
            .query_filtered::<Entity, With<BlasterShot>>()
            .single(harness.client.world())
            .expect("immediate local shot");
        harness.frames(60);
        let shots: Vec<_> = harness
            .client
            .world_mut()
            .query_filtered::<Entity, With<BlasterShot>>()
            .iter(harness.client.world())
            .collect();
        assert_eq!(
            shots,
            [local_shot],
            "authoritative shot must match the prespawn (loss={loss}, abrupt={abrupt})"
        );
        assert!(harness.client.world().get::<Remote>(local_shot).is_some());
        harness.frames(200);
        assert_eq!(
            harness
                .client
                .world_mut()
                .query_filtered::<Entity, With<BlasterShot>>()
                .iter(harness.client.world())
                .count(),
            0
        );
        assert_eq!(
            harness
                .server
                .world_mut()
                .query_filtered::<Entity, With<BlasterShot>>()
                .iter(harness.server.world())
                .count(),
            0
        );
    }
}

fn client_shot_count(harness: &mut Harness) -> usize {
    harness
        .client
        .world_mut()
        .query_filtered::<Entity, With<BlasterShot>>()
        .iter(harness.client.world())
        .count()
}

#[test]
fn projectile_visibility_hides_reveals_and_preserves_owned_shots() {
    use space_game_protocol::BlasterTrajectory;

    let mut harness = Harness::new(true);
    harness.frames(600);
    let remote = harness
        .server
        .world_mut()
        .spawn((
            BlasterShot {
                position: Vec2::splat(7000.0),
                ticks_left: 250,
            },
            BlasterTrajectory {
                direction: Vec2::ZERO,
            },
            PreSpawned::new(999),
        ))
        .id();
    harness.frames(70);
    assert_eq!(
        client_shot_count(&mut harness),
        0,
        "hidden before first replication"
    );
    harness
        .server
        .world_mut()
        .get_mut::<BlasterShot>(remote)
        .expect("shot")
        .position = Vec2::new(100.0, 0.0);
    harness.frames(70);
    assert_eq!(
        client_shot_count(&mut harness),
        1,
        "visible remote shot is predicted after reveal"
    );
    harness
        .server
        .world_mut()
        .get_mut::<BlasterShot>(remote)
        .expect("shot")
        .position = Vec2::splat(7000.0);
    harness.frames(70);
    assert_eq!(
        client_shot_count(&mut harness),
        0,
        "hidden copies are removed, not retained stale"
    );
    harness.server.world_mut().spawn((
        BlasterShot {
            position: Vec2::splat(7000.0),
            ticks_left: 250,
        },
        BlasterTrajectory {
            direction: Vec2::ZERO,
        },
        PreSpawned::new(1000).for_client(harness.server_link),
    ));
    harness.frames(70);
    assert_eq!(
        client_shot_count(&mut harness),
        1,
        "owner always receives its authoritative shot"
    );
    let player = harness.controlled_player();
    assert!(harness.client.world().get::<Position>(player).is_some());
}
