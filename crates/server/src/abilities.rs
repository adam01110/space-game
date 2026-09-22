use avian2d::prelude::Position;
use bevy::{platform::collections::HashMap, prelude::*};
use lightyear::prelude::{server::ClientOf, *};
use space_game_protocol::{BlasterShot, Player};

// Transport-independent so headless network tests use the production policy.
pub struct ServerAbilitiesPlugin;

impl Plugin for ServerAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectileInterest>()
            .init_resource::<ProjectileVisibility>()
            .add_observer(replicate_blaster_shot)
            // Fixed simulation creates/moves shots before Update; replication sends
            // in PostUpdate. New shots and new connections are filtered immediately.
            .add_systems(Update, update_projectile_visibility);
    }
}

// A conservative world-space interest envelope, not a client-provided viewport.
// Solid players and arena state must remain global for same-tick collision prediction.
#[derive(Resource)]
pub(crate) struct ProjectileInterest {
    enter: f32,
    exit: f32,
}

impl Default for ProjectileInterest {
    fn default() -> Self {
        Self {
            enter: 4096.0,
            exit: 4608.0,
        }
    }
}

#[derive(Resource, Default)]
struct ProjectileVisibility(HashMap<(Entity, Entity), bool>);

fn replicate_blaster_shot(
    trigger: On<Add, BlasterShot>,
    shots: Query<&PreSpawned, With<BlasterShot>>,
    mut commands: Commands,
) {
    let mut entity = commands.entity(trigger.entity);
    entity.insert((
        Replicate::to_clients(NetworkTarget::All),
        // Projectiles and colliders share the same predicted timeline. Remote
        // clients no longer create speculative prespawns from rebroadcast inputs.
        PredictionTarget::to_clients(NetworkTarget::All),
        // Affects mutation eligibility, not reliable spawn/despawn delivery.
        ReplicatePriority(0.5),
    ));
    if let Some(owner) = shots
        .get(trigger.entity)
        .ok()
        .and_then(|prespawn| prespawn.client)
    {
        entity.insert(ControlledBy {
            owner,
            lifetime: default(),
        });
    }
}

pub(crate) fn interested(
    was_visible: bool,
    owner: bool,
    distance_squared: f32,
    policy: &ProjectileInterest,
) -> bool {
    let radius = if was_visible {
        policy.exit
    } else {
        policy.enter
    };
    owner || distance_squared <= radius * radius
}

fn update_projectile_visibility(
    mut commands: Commands,
    policy: Res<ProjectileInterest>,
    mut cache: ResMut<ProjectileVisibility>,
    players: Query<(&Position, &ControlledBy), With<Player>>,
    clients: Query<Entity, (With<ClientOf>, With<Connected>)>,
    shots: Query<(Entity, &BlasterShot, Option<&ControlledBy>), With<Replicate>>,
) {
    cache
        .0
        .retain(|(shot, client), _| shots.contains(*shot) && clients.contains(*client));
    for client in &clients {
        let origin = players
            .iter()
            .find(|(_, control)| control.owner == client)
            .map(|(position, _)| position.0);
        for (entity, shot, control) in &shots {
            let previous = cache.0.get(&(entity, client)).copied();
            let visible = interested(
                previous.unwrap_or(false),
                control.is_some_and(|control| control.owner == client),
                origin.map_or(f32::INFINITY, |origin| {
                    origin.distance_squared(shot.position)
                }),
                &policy,
            );
            if previous != Some(visible) {
                if visible {
                    commands.gain_visibility(entity, client);
                } else {
                    commands.lose_visibility(entity, client);
                }
                cache.0.insert((entity, client), visible);
            }
        }
    }
}
