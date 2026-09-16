use avian2d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::{ecs::component::Mutable, prelude::*};
use lightyear::prelude::*;

use project_protocol::{
    ArenaBoundary, BlasterReload, BlasterShot, BlasterTrigger, PlayerBlasters, PlayerBoost,
    PlayerHealth, PlayerPhaseBeam,
};

struct ConfirmedComponent {
    available: fn(&mut World, Tick) -> bool,
    apply: fn(&mut World, Tick),
}

impl ConfirmedComponent {
    const fn new<C: Component<Mutability = Mutable> + Clone + PartialEq>() -> Self {
        Self {
            available: has_confirmed_state::<C>,
            apply: apply_confirmed_state::<C>,
        }
    }
}

// The same component set controls both passive presentation and safe prediction recovery. Keep
// aligned with ProtocolPlugin and LightyearAvianPlugin's replicated predicted state.
const PREDICTED_COMPONENTS: &[ConfirmedComponent] = &[
    ConfirmedComponent::new::<Position>(),
    ConfirmedComponent::new::<Rotation>(),
    ConfirmedComponent::new::<LinearVelocity>(),
    ConfirmedComponent::new::<AngularVelocity>(),
    ConfirmedComponent::new::<ArenaBoundary>(),
    ConfirmedComponent::new::<PlayerBlasters>(),
    ConfirmedComponent::new::<BlasterTrigger>(),
    ConfirmedComponent::new::<BlasterReload>(),
    ConfirmedComponent::new::<BlasterShot>(),
    ConfirmedComponent::new::<PlayerBoost>(),
    ConfirmedComponent::new::<PlayerHealth>(),
    ConfirmedComponent::new::<PlayerPhaseBeam>(),
];

// Whether every predicted component can be restored from history at `tick`.
pub(super) fn has_confirmed_world_state(world: &mut World, tick: Tick) -> bool {
    PREDICTED_COMPONENTS
        .iter()
        .all(|component| (component.available)(world, tick))
}

// Restore every predicted component from its history at `tick`.
pub(super) fn apply_confirmed_world_state(world: &mut World, tick: Tick) {
    for component in PREDICTED_COMPONENTS {
        (component.apply)(world, tick);
    }
}

// A completed checkpoint proves that unchanged components retain their previous value. Missing
// history is not an explicit removal and must never enable stale prediction.
fn has_confirmed_state<C: Component>(world: &mut World, tick: Tick) -> bool {
    world
        .query_filtered::<Option<&ConfirmedHistory<C>>, (
            With<Predicted>,
            Or<(With<C>, With<ConfirmedHistory<C>>)>,
            Allow<PredictionDisable>,
        )>()
        .iter(world)
        .all(|history| {
            history.is_some_and(|history| history.get_state_at_or_before(tick).is_some())
        })
}

fn apply_confirmed_state<C: Component<Mutability = Mutable> + Clone + PartialEq>(
    world: &mut World,
    tick: Tick,
) {
    let states: Vec<_> = world
        .query_filtered::<(Entity, &ConfirmedHistory<C>), With<Predicted>>()
        .iter(world)
        .filter_map(|(entity, history)| {
            history
                .get_state_at_or_before(tick)
                .cloned()
                .map(|state| (entity, state))
        })
        .collect();

    for (entity, state) in states {
        set_confirmed_state::<C>(&mut world.entity_mut(entity), state);
    }
}

// Write one confirmed value back, or remove the component when it was confirmed absent.
fn set_confirmed_state<C: Component<Mutability = Mutable> + Clone + PartialEq>(
    entity: &mut EntityWorldMut,
    state: HistoryState<C>,
) {
    match state {
        HistoryState::Updated(value) => {
            entity
                .entry::<C>()
                .or_insert_with(|| value.clone())
                .into_mut()
                .set_if_neq(value);
        }
        HistoryState::Removed => {
            entity.remove::<C>();
        }
    }
}

#[cfg(test)]
#[path = "../tests/confirmed.rs"]
mod tests;
