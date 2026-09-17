mod presentation;
mod restoration;

use avian2d::prelude::PhysicsSystems;
use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::{frame_interpolation::FrameInterpolationSystems, prelude::*};

use space_game_game::ClientSimulationSystems;

use restoration::observe_restoration;
pub(crate) use restoration::{finish_restoration, request_restoration};

// Only tests call the checkpoint budget directly; production code reaches it through
// `request_restoration`.
#[cfg(test)]
pub(crate) use restoration::recent_checkpoint;

pub(super) struct ClientFocusPlugin;

// Prediction availability, driven by window focus.
#[derive(Resource, Default, Debug)]
pub(super) struct FocusPrediction {
    pub(super) phase: PredictionPhase,
}

#[derive(Default, Debug)]
pub(super) enum PredictionPhase {
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
    pub(super) const fn await_new_checkpoint(&mut self, previous_checkpoint: Option<Tick>) {
        self.phase = PredictionPhase::Waiting {
            previous_checkpoint,
        };
    }
}

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
                presentation::present_authoritative_state
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
pub(super) fn observe_focus(
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
