use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{Predicted, SyncedLocalTimeline, input::native::ActionState};

use crate::PLAYER_RADIUS;
use project_protocol::{Player, PlayerInput};

pub const BEAM_LENGTH: f32 = 800.0;
pub const BEAM_WIDTH: f32 = 8.0;
pub const NOSE_OFFSET: f32 = PLAYER_RADIUS + 4.0;

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

pub(super) fn beam_predicted_players(
    _timeline: SyncedLocalTimeline,
    mut commands: Commands,
    mut players: Query<BeamState, With<Predicted>>,
) {
    for (entity, position, rotation, input, segment) in &mut players {
        sync_beam(
            &mut commands,
            entity,
            position,
            rotation,
            input.0.phase_beam,
            segment,
        );
    }
}

pub(super) fn beam_authoritative_players(
    mut commands: Commands,
    mut players: Query<BeamState, With<Player>>,
) {
    for (entity, position, rotation, input, segment) in &mut players {
        sync_beam(
            &mut commands,
            entity,
            position,
            rotation,
            input.0.phase_beam,
            segment,
        );
    }
}
