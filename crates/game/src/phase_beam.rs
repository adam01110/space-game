use avian2d::prelude::{Position, Rotation};
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};

use crate::PLAYER_RADIUS;
use project_protocol::{Player, PlayerInput, PlayerPhaseBeam};

pub const BEAM_LENGTH: f32 = 800.0;
pub const BEAM_WIDTH: f32 = 8.0;
pub const NOSE_OFFSET: f32 = PLAYER_RADIUS + 4.0;

const CHARGE_DRAIN_PER_SECOND: u8 = 25;
const CHARGE_REGEN_PER_SECOND: u8 = 25;

/*
Beam segment in world space, published each tick for hit detection and rendering.
Present only while the beam is held; removal marks the beam inactive.
*/
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PhaseBeamSegment {
    pub origin: Vec2,
    pub direction: Vec2,
}

impl PhaseBeamSegment {
    #[must_use]
    pub fn end(self) -> Vec2 {
        self.origin + self.direction * BEAM_LENGTH
    }
}

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
            component.origin = segment_origin(position, rotation);
            component.direction = segment_direction(rotation);
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
        let active = match input.0.phase_beam {
            true => charge.0.drain(CHARGE_DRAIN_PER_SECOND, delta),
            false => {
                charge.0.regenerate(CHARGE_REGEN_PER_SECOND, delta);
                false
            }
        };
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
