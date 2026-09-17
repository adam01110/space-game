use bevy::prelude::*;
use lightyear::prelude::*;

use crate::confirmed::has_confirmed_world_state;

use super::presentation::{VisualCorrections, clear_visual_corrections};
use super::{FocusPrediction, PredictionPhase};

// The checkpoint to roll back to, once one is usable.
fn restoration_tick(world: &World) -> Option<Tick> {
    let PredictionPhase::Waiting {
        previous_checkpoint,
    } = world.resource::<FocusPrediction>().phase
    else {
        return None;
    };

    if !world.resource::<LocalTimelineSync>().is_synced() {
        return None;
    }

    let manager = world.get_resource::<PredictionManager>()?;
    let max_depth = manager
        .rollback_policy
        .effective_max_rollback_ticks(world.resource::<InputTimelineConfig>());

    recent_checkpoint(
        previous_checkpoint,
        world
            .resource::<ReplicationCheckpointMap>()
            .last_confirmed_tick(),
        world.resource::<LocalTimeline>().tick(),
        max_depth,
    )
}

// A checkpoint is usable when it arrived after focus returned and is recent enough to replay
// within the configured rollback budget.
// `Tick` subtraction clamps, so a checkpoint ahead of the local timeline yields a negative depth
// and is rejected by the budget check as well.
pub(crate) fn recent_checkpoint(
    previous: Option<Tick>,
    checkpoint: Option<Tick>,
    local: Tick,
    max_depth: u16,
) -> Option<Tick> {
    let checkpoint = checkpoint?;

    if Some(checkpoint) == previous {
        return None;
    }

    let depth = local - checkpoint;
    (0..=i32::from(max_depth))
        .contains(&depth)
        .then_some(checkpoint)
}

// Ask Lightyear for a forced rollback to the newest usable checkpoint.
pub(crate) fn request_restoration(world: &mut World) {
    let Some(tick) = restoration_tick(world) else {
        return;
    };

    if !has_confirmed_world_state(world, tick) {
        return;
    }

    world
        .resource_mut::<StateRollbackMetadata>()
        .request_forced_rollback(tick);

    world.resource_mut::<FocusPrediction>().phase = PredictionPhase::Restoring {
        tick,
        prepared: false,
    };
}

// Record whether Lightyear actually prepared the rollback we asked for.
// Consuming the request is not proof that the rollback ran.
pub(super) fn observe_restoration(
    rollback: Option<Res<Rollback>>,
    manager: Option<Res<PredictionManager>>,
    mut state: ResMut<FocusPrediction>,
) {
    if let PredictionPhase::Restoring { tick, prepared } = &mut state.phase {
        *prepared = rollback_prepared(rollback.as_deref(), manager.as_deref(), *tick);
    }
}

// Whether Lightyear prepared the forced rollback we asked for, starting at exactly `tick`.
fn rollback_prepared(
    rollback: Option<&Rollback>,
    manager: Option<&PredictionManager>,
    tick: Tick,
) -> bool {
    matches!(rollback, Some(Rollback::FromState))
        && manager.is_some_and(|manager| manager.get_rollback_start_tick() == Some(tick))
}

// The forced rollback we requested has run to completion: it was prepared, and Lightyear has
// since cleared its rollback state.
const fn rollback_finished(prepared: bool, rollback: Option<&Rollback>) -> bool {
    prepared && rollback.is_none()
}

// Resume prediction once the rollback has completed, or wait for another checkpoint if it did
// not.
pub(crate) fn finish_restoration(
    rollback: Option<Res<Rollback>>,
    checkpoints: Res<ReplicationCheckpointMap>,
    mut state: ResMut<FocusPrediction>,
    corrections: Query<Entity, VisualCorrections>,
    mut commands: Commands,
) {
    let PredictionPhase::Restoring { tick, prepared } = state.phase else {
        return;
    };

    match rollback_finished(prepared, rollback.as_deref()) {
        true => {
            clear_visual_corrections(&mut commands, &corrections);
            debug!(?tick, "Resumed client prediction from authoritative state");
            state.phase = PredictionPhase::Active;
        }
        false => {
            // A rejected or skipped rollback must not re-enable stale simulation.
            state.await_new_checkpoint(checkpoints.last_confirmed_tick());
        }
    }
}
