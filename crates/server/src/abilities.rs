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
            // Fixed simulation creates/moves shots before `Update`, replication sends in
            // `PostUpdate`; new shots and connections are filtered immediately.
            .add_systems(Update, update_projectile_visibility);
    }
}

// A conservative world-space interest envelope, not a client-provided viewport. Solid players and
// arena state stay global for same-tick collision prediction.
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
        // Projectiles and colliders share the same predicted timeline; remote clients no longer
        // create speculative prespawns from rebroadcast inputs.
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
    let radius = match was_visible {
        true => policy.exit,
        false => policy.enter,
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
        let origin = owner_position(&players, client);

        for (entity, shot, control) in &shots {
            let key = (entity, client);
            let owner = control.is_some_and(|control| control.owner == client);
            let Some(visible) = visibility_change(
                cache.0.get(&key).copied(),
                owner,
                shot_distance(origin, shot.position),
                &policy,
            ) else {
                continue;
            };

            apply_visibility(&mut commands, entity, client, visible);
            cache.0.insert(key, visible);
        }
    }
}

fn owner_position(
    players: &Query<(&Position, &ControlledBy), With<Player>>,
    client: Entity,
) -> Option<Vec2> {
    players
        .iter()
        .find(|(_, control)| control.owner == client)
        .map(|(position, _)| position.0)
}

fn shot_distance(origin: Option<Vec2>, position: Vec2) -> f32 {
    origin.map_or(f32::INFINITY, |origin| origin.distance_squared(position))
}

fn visibility_change(
    previous: Option<bool>,
    owner: bool,
    distance_squared: f32,
    policy: &ProjectileInterest,
) -> Option<bool> {
    let visible = interested(previous.unwrap_or(false), owner, distance_squared, policy);
    (previous != Some(visible)).then_some(visible)
}

fn apply_visibility(commands: &mut Commands, entity: Entity, client: Entity, visible: bool) {
    match visible {
        true => commands.gain_visibility(entity, client),
        false => commands.lose_visibility(entity, client),
    }
}
