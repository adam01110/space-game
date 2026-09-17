use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use lightyear::prelude::{ReplicationCheckpointMap, VisualCorrection};

use crate::confirmed::apply_confirmed_world_state;

pub(super) type VisualCorrections = Or<(
    With<VisualCorrection<Position>>,
    With<VisualCorrection<Rotation>>,
)>;

// Keep authoritative server snapshots visible while prediction and local control are paused.
// This runs on both native and WASM; focus loss is not the same as being invisible.
pub(super) fn present_authoritative_state(world: &mut World) {
    let Some(tick) = world
        .resource::<ReplicationCheckpointMap>()
        .last_confirmed_tick()
    else {
        return;
    };

    apply_confirmed_world_state(world, tick);
}

// Drop the correction animation that would otherwise interpolate from the background pose to the
// live world. This is a resynchronization, not a small prediction error, so it should snap.
pub(super) fn clear_visual_corrections(
    commands: &mut Commands,
    corrections: &Query<Entity, VisualCorrections>,
) {
    for entity in corrections {
        commands
            .entity(entity)
            .remove::<(VisualCorrection<Position>, VisualCorrection<Rotation>)>();
    }
}
