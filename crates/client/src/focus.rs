use avian2d::prelude::{PhysicsSystems, Position, Rotation};
use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::{frame_interpolation::FrameInterpolationSystems, prelude::*};

use project_game::ClientSimulationSystems;

use crate::confirmed::{apply_confirmed_world_state, has_confirmed_world_state};

pub(super) struct ClientFocusPlugin;

// Prediction availability, driven by window focus.
#[derive(Resource, Default, Debug)]
pub(super) struct FocusPrediction {
    phase: PredictionPhase,
}

#[derive(Default, Debug)]
enum PredictionPhase {
    // Focused: simulation, prediction and input all run normally.
    #[default]
    Active,
    // Unfocused: local simulation is gated and only authoritative state is presented.
    Suspended,
    // Focused again, waiting for a checkpoint received after focus was lost.
    Waiting {
        previous_checkpoint: Option<Tick>,
    },
    // Rollback to an authoritative checkpoint has been requested but has not completed.
    Restoring {
        tick: Tick,
        prepared: bool,
    },
}

impl FocusPrediction {
    // Local controls are only meaningful while prediction is active.
    pub(super) const fn accepts_input(&self) -> bool {
        matches!(self.phase, PredictionPhase::Active)
    }

    // Stop simulating, logging only on the transition into suspension.
    fn suspend(&mut self) {
        if matches!(self.phase, PredictionPhase::Suspended) {
            return;
        }

        debug!("Suspending client prediction while unfocused");
        self.phase = PredictionPhase::Suspended;
    }

    // Focus regained: wait for a checkpoint received after focus returns rather than the
    // focus-loss state.
    const fn begin_recovery(&mut self, previous_checkpoint: Option<Tick>) {
        if matches!(self.phase, PredictionPhase::Suspended) {
            self.await_new_checkpoint(previous_checkpoint);
        }
    }

    // Wait for a checkpoint newer than `previous_checkpoint` before resuming simulation.
    const fn await_new_checkpoint(&mut self, previous_checkpoint: Option<Tick>) {
        self.phase = PredictionPhase::Waiting {
            previous_checkpoint,
        };
    }
}

type VisualCorrections = Or<(
    With<VisualCorrection<Position>>,
    With<VisualCorrection<Rotation>>,
)>;

// Local gameplay may only advance while prediction is active or being restored.
fn simulating(state: Res<FocusPrediction>) -> bool {
    matches!(
        state.phase,
        PredictionPhase::Active | PredictionPhase::Restoring { .. }
    )
}

impl Plugin for ClientFocusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusPrediction>()
            .add_systems(
                PreUpdate,
                observe_focus
                    .before(MessageSystems::Receive)
                    .before(ReplicationSystems::Receive)
                    .before(crate::input::capture_ability_inputs),
            )
            .add_systems(
                PreUpdate,
                request_restoration
                    .after(ReplicationSystems::Receive)
                    .before(PredictionSystems::All),
            )
            .add_systems(
                PreUpdate,
                observe_restoration
                    .after(RollbackSystems::Prepare)
                    .before(RollbackSystems::Rollback),
            )
            .add_systems(PreUpdate, finish_restoration.after(PredictionSystems::All))
            .add_systems(
                PreUpdate,
                present_authoritative_state
                    .after(finish_restoration)
                    .run_if(not(simulating)),
            );

        gate_simulation_sets(app);
    }
}

// Disable every set that would advance the local timeline while prediction is suspended.
// Input buffering, transport, replication and timeline synchronization stay enabled: they still
// run at the throttled background cadence and provide the authoritative snapshots that
// restoration depends on. Frame interpolation is gated too, because its stale predicted frame
// histories would overwrite the authoritative poses presented while suspended.
fn gate_simulation_sets(app: &mut App) {
    app.configure_sets(
        RunFixedMainLoop,
        FrameInterpolationSystems::Restore.run_if(simulating),
    )
    .configure_sets(
        FixedPostUpdate,
        FrameInterpolationSystems::Update.run_if(simulating),
    )
    .configure_sets(
        PostUpdate,
        FrameInterpolationSystems::Interpolate.run_if(simulating),
    );

    app.configure_sets(PreUpdate, PredictionSystems::All.run_if(simulating))
        .configure_sets(FixedPreUpdate, PredictionSystems::All.run_if(simulating))
        .configure_sets(FixedPostUpdate, PredictionSystems::All.run_if(simulating))
        .configure_sets(PostUpdate, PredictionSystems::All.run_if(simulating))
        .configure_sets(FixedUpdate, ClientSimulationSystems.run_if(simulating))
        .configure_sets(FixedPostUpdate, ClientSimulationSystems.run_if(simulating))
        .configure_sets(
            FixedPostUpdate,
            PhysicsSystems::StepSimulation.run_if(simulating),
        );
}

// Suspend prediction on focus loss and start recovery on focus regain.
fn observe_focus(
    windows: Query<&Window, With<PrimaryWindow>>,
    checkpoints: Res<ReplicationCheckpointMap>,
    mut state: ResMut<FocusPrediction>,
) {
    if windows.single().is_ok_and(|window| window.focused) {
        state.begin_recovery(checkpoints.last_confirmed_tick());
        return;
    }

    state.suspend();
}

// Keep authoritative server snapshots visible while prediction and local control are paused.
// This runs on both native and WASM; focus loss is not the same as being invisible.
fn present_authoritative_state(world: &mut World) {
    let Some(tick) = world
        .resource::<ReplicationCheckpointMap>()
        .last_confirmed_tick()
    else {
        return;
    };

    apply_confirmed_world_state(world, tick);
}

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
fn recent_checkpoint(
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
fn request_restoration(world: &mut World) {
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
fn observe_restoration(
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

// Drop the correction animation that would otherwise interpolate from the background pose to the
// live world. This is a resynchronization, not a small prediction error, so it should snap.
fn clear_visual_corrections(
    commands: &mut Commands,
    corrections: &Query<Entity, VisualCorrections>,
) {
    for entity in corrections {
        commands
            .entity(entity)
            .remove::<(VisualCorrection<Position>, VisualCorrection<Rotation>)>();
    }
}

// Resume prediction once the rollback has completed, or wait for another checkpoint if it did
// not.
fn finish_restoration(
    rollback: Option<Res<Rollback>>,
    checkpoints: Res<ReplicationCheckpointMap>,
    mut state: ResMut<FocusPrediction>,
    corrections: Query<Entity, VisualCorrections>,
    mut commands: Commands,
) {
    let PredictionPhase::Restoring { tick, prepared } = state.phase else {
        return;
    };

    if !rollback_finished(prepared, rollback.as_deref()) {
        // A rejected or skipped rollback must not re-enable stale simulation.
        state.await_new_checkpoint(checkpoints.last_confirmed_tick());
        return;
    }

    clear_visual_corrections(&mut commands, &corrections);
    debug!(?tick, "Resumed client prediction from authoritative state");
    state.phase = PredictionPhase::Active;
}

#[cfg(test)]
#[path = "../tests/focus.rs"]
mod tests;
