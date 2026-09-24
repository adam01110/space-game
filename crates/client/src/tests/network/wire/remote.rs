use super::*;

struct Observer {
    app: App,
    server_link: Entity,
    client_link: Entity,
    upstream: Wire,
    downstream: Wire,
}

impl Observer {
    fn new(harness: &mut Harness) -> Self {
        let mut app = App::new();
        app.add_plugins((
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
        app.insert_resource(policy::timeline_config());
        app.insert_resource(policy::prediction_manager());
        app.init_resource::<ScriptedInput>();
        app.add_observer(mark_controlled);
        app.add_systems(
            FixedPreUpdate,
            write_input.in_set(lightyear::prelude::client::input::InputSystems::WriteClientInputs),
        );
        app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        app.insert_resource(ReplicationMetadata::new(TICK));
        app.finish();
        app.cleanup();
        app.update();
        let root = harness
            .server
            .world()
            .get::<LinkOf>(harness.server_link)
            .expect("root")
            .server;
        let server_link = harness
            .server
            .world_mut()
            .spawn((
                Link::default(),
                LinkOf { server: root },
                ClientOf,
                LocalId(PeerId::Server),
                RemoteId(PeerId::Local(2)),
                ReplicationSender,
                PingManager::default(),
                Linked,
            ))
            .id();
        harness
            .server
            .world_mut()
            .entity_mut(server_link)
            .insert(Connected);
        let client_link = app
            .world_mut()
            .spawn((
                Link::default(),
                Client,
                LocalId(PeerId::Local(2)),
                RemoteId(PeerId::Server),
                ReplicationReceiver,
                PingManager::default(),
                Linked,
            ))
            .id();
        app.world_mut().entity_mut(client_link).insert(Connected);
        harness.server.world_mut().spawn((
            PlayerBundle::new(Vec2::new(200.0, 0.0)),
            PlayerIdentity(8),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::All),
            ControlledBy {
                owner: server_link,
                lifetime: default(),
            },
        ));
        Self {
            app,
            server_link,
            client_link,
            upstream: Wire::default(),
            downstream: Wire::default(),
        }
    }

    fn step(&mut self, harness: &mut Harness) {
        let now = harness.now + FRAME;
        self.upstream
            .deliver(&mut harness.server, self.server_link, now);
        self.downstream
            .deliver(&mut self.app, self.client_link, now);
        harness.step();
        self.app.update();
        self.upstream.collect(&mut self.app, self.client_link, now);
        self.downstream
            .collect(&mut harness.server, self.server_link, now);
    }

    fn frames(&mut self, harness: &mut Harness, count: usize) {
        for _ in 0..count {
            self.step(harness);
        }
    }

    fn shooter_position(&mut self) -> Vec2 {
        self.app
            .world_mut()
            .query::<(&Position, &PlayerIdentity)>()
            .iter(self.app.world())
            .find(|(_, id)| id.0 == 7)
            .expect("remote shooter")
            .0
             .0
    }
}

#[test]
fn remote_shots_first_appear_near_the_visible_shooter_without_netem() {
    let mut harness = Harness::new(false);
    harness.downstream.delay = Duration::ZERO;
    let mut observer = Observer::new(&mut harness);
    observer.frames(&mut harness, 600);
    harness
        .client
        .world_mut()
        .resource_mut::<ScriptedInput>()
        .0
        .blaster_clicks = 1;
    let mut first_distance = None;
    for _ in 0..100 {
        observer.step(&mut harness);
        let position = observer
            .app
            .world_mut()
            .query::<&BlasterShot>()
            .iter(observer.app.world())
            .next()
            .map(|shot| shot.position);
        if let Some(position) = position {
            first_distance = Some(position.distance(observer.shooter_position()));
            break;
        }
    }
    let distance = first_distance.expect("observer saw shot");
    // Muzzle offset is 31.4; allow at most two 20-unit simulation steps.
    assert!(
        distance <= 72.0,
        "first remote shot appeared {distance} units from the shooter"
    );
}

#[test]
fn predicted_contact_does_not_cross_or_overlap_during_motion() {
    let mut harness = Harness::new(false);
    harness.downstream.delay = Duration::ZERO;
    let mut observer = Observer::new(&mut harness);
    observer.frames(&mut harness, 600);
    harness
        .client
        .world_mut()
        .resource_mut::<ScriptedInput>()
        .0
        .movement = Vec2::X;
    observer
        .app
        .world_mut()
        .resource_mut::<ScriptedInput>()
        .0
        .movement = Vec2::NEG_X;
    let mut min_separation = f32::INFINITY;
    for frame in 0..240 {
        if frame == 80 {
            harness
                .client
                .world_mut()
                .resource_mut::<ScriptedInput>()
                .0
                .movement = Vec2::ZERO;
            observer
                .app
                .world_mut()
                .resource_mut::<ScriptedInput>()
                .0
                .movement = Vec2::ZERO;
        }
        observer.step(&mut harness);
        for app in [&mut harness.client, &mut observer.app] {
            let positions: Vec<_> = app
                .world_mut()
                .query_filtered::<(&Position, &PlayerIdentity), With<Predicted>>()
                .iter(app.world())
                .map(|(position, id)| (id.0, position.0))
                .collect();
            let left = positions.iter().find(|(id, _)| *id == 7).expect("left").1;
            let right = positions.iter().find(|(id, _)| *id == 8).expect("right").1;
            assert!(
                left.x < right.x,
                "players crossed at frame {frame}: {left:?}, {right:?}"
            );
            min_separation = min_separation.min(left.distance(right));
        }
    }
    assert!(
        min_separation >= 45.0,
        "predicted collision separation fell to {min_separation}"
    );
}
