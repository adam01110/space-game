use avian2d::prelude::{Position, Rotation};
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};

use space_game_protocol::{PhaseBeamSegment, Player, PlayerInput, PlayerPhaseBeam};

use crate::PLAYER_RADIUS;

pub const BEAM_LENGTH: f32 = 400.0;
pub const BEAM_WIDTH: f32 = 8.0;
pub const NOSE_OFFSET: f32 = PLAYER_RADIUS + 4.0;

const CHARGE_DRAIN_PER_SECOND: u8 = 40;
const CHARGE_REGEN_PER_SECOND: u8 = 25;

fn segment_origin(position: &Position, rotation: &Rotation) -> Vec2 {
    position.0 + *rotation * (Vec2::Y * NOSE_OFFSET)
}

fn segment_direction(rotation: &Rotation) -> Vec2 {
    *rotation * Vec2::Y
}

type BeamState<'a> = (
    Entity,
    &'a Position,
    &'a Rotation,
    &'a ActionState<PlayerInput>,
    &'a mut PlayerPhaseBeam,
    Option<&'a mut PhaseBeamSegment>,
);

// Keeps the segment component in sync with the held input, including rollback.
fn sync_beam(
    commands: &mut Commands,
    entity: Entity,
    position: &Position,
    rotation: &Rotation,
    active: bool,
    segment: Option<Mut<PhaseBeamSegment>>,
) {
    if !active {
        if segment.is_some() {
            commands.entity(entity).remove::<PhaseBeamSegment>();
        }
        return;
    }

    match segment {
        Some(mut component) => {
            component.set_if_neq(PhaseBeamSegment {
                origin: segment_origin(position, rotation),
                direction: segment_direction(rotation),
            });
        }
        None => {
            commands.entity(entity).insert(PhaseBeamSegment {
                origin: segment_origin(position, rotation),
                direction: segment_direction(rotation),
            });
        }
    }
}

fn update_beams(
    commands: &mut Commands,
    players: &mut Query<BeamState, impl QueryFilter>,
    delta: std::time::Duration,
) {
    for (entity, position, rotation, input, mut charge, segment) in players {
        let mut next_charge = *charge;
        let active = match input.0.phase_beam {
            true => next_charge.0.drain(CHARGE_DRAIN_PER_SECOND, delta),
            false => {
                next_charge.0.regenerate(CHARGE_REGEN_PER_SECOND, delta);
                false
            }
        };

        charge.set_if_neq(next_charge);
        sync_beam(commands, entity, position, rotation, active, segment);
    }
}

pub(super) fn beam_predicted_players(
    _timeline: SyncedLocalTimeline,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut players: Query<BeamState, With<Predicted>>,
) {
    update_beams(&mut commands, &mut players, time.delta());
}

pub(super) fn beam_authoritative_players(
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut players: Query<BeamState, With<Player>>,
) {
    update_beams(&mut commands, &mut players, time.delta());
}
