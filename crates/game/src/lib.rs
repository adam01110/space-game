use bevy::prelude::*;
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};
use project_protocol::{Player, PlayerHeading, PlayerInput, PlayerPosition};

const MOVE_SPEED: f32 = 500.0;
const TURN_SPEED: f32 = 6.0;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_player_transforms);
    }
}

pub struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_predicted_players);
    }
}

pub struct ServerSimulationPlugin;

impl Plugin for ServerSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_authoritative_players);
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub heading: PlayerHeading,
    pub player: Player,
    pub position: PlayerPosition,
}

impl PlayerBundle {
    pub fn new(position: Vec2) -> Self {
        Self {
            heading: PlayerHeading::default(),
            player: Player,
            position: PlayerPosition(position),
        }
    }
}

/// Runs prediction only after Lightyear has synchronized the client timeline.
fn move_predicted_players(
    _timeline: SyncedLocalTimeline,
    time: Res<Time<Fixed>>,
    mut players: Query<
        (
            &mut PlayerPosition,
            &mut PlayerHeading,
            &ActionState<PlayerInput>,
        ),
        (With<Player>, With<Predicted>),
    >,
) {
    let delta_seconds = time.delta_secs();
    for (position, heading, input) in &mut players {
        apply_movement(position, heading, input, delta_seconds);
    }
}

fn move_authoritative_players(
    time: Res<Time<Fixed>>,
    mut players: Query<
        (
            &mut PlayerPosition,
            &mut PlayerHeading,
            &ActionState<PlayerInput>,
        ),
        With<Player>,
    >,
) {
    let delta_seconds = time.delta_secs();
    for (position, heading, input) in &mut players {
        apply_movement(position, heading, input, delta_seconds);
    }
}

/// Shared simulation used by the authoritative server and predicted clients.
fn apply_movement(
    mut position: Mut<PlayerPosition>,
    mut heading: Mut<PlayerHeading>,
    input: &ActionState<PlayerInput>,
    delta_seconds: f32,
) {
    let input = input.0;

    if let Some(aim) = input.aim.try_normalize() {
        let target = aim.y.atan2(aim.x) - std::f32::consts::FRAC_PI_2;
        heading.0 = turn_towards(heading.0, target, TURN_SPEED * delta_seconds);
    }

    let velocity = input.movement.clamp_length_max(1.0) * MOVE_SPEED;
    position.0 += velocity * delta_seconds;
}

fn sync_player_transforms(
    mut players: Query<(&PlayerPosition, &PlayerHeading, &mut Transform), With<Player>>,
) {
    for (position, heading, mut transform) in &mut players {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
        transform.rotation = Quat::from_rotation_z(heading.0);
    }
}

fn turn_towards(current: f32, target: f32, max_step: f32) -> f32 {
    let difference = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    current + difference.clamp(-max_step, max_step)
}
