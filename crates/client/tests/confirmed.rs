use super::*;
use bevy::ecs::component::Mutable;

fn assert_history_required<C: Component<Mutability = Mutable> + Clone + PartialEq>(value: C) {
    let mut world = World::new();
    let entity = world.spawn((Predicted, value.clone())).id();
    let tick = Tick(10);
    assert!(!has_confirmed_world_state(&mut world, tick));

    let mut history = ConfirmedHistory::<C>::default();
    history.insert_present(Tick(11), value.clone());
    world.entity_mut(entity).insert(history);
    assert!(!has_confirmed_world_state(&mut world, tick));

    world
        .get_mut::<ConfirmedHistory<C>>(entity)
        .expect("history")
        .insert_present(tick, value.clone());
    assert!(has_confirmed_world_state(&mut world, tick));
    world.entity_mut(entity).remove::<C>();
    apply_confirmed_world_state(&mut world, tick);
    assert!(world.get::<C>(entity) == Some(&value));
    world
        .get_mut::<ConfirmedHistory<C>>(entity)
        .expect("history")
        .insert_removed(tick);
    assert!(has_confirmed_world_state(&mut world, tick));
    apply_confirmed_world_state(&mut world, tick);
    assert!(world.get::<C>(entity).is_none());
}

#[test]
fn every_predicted_component_requires_usable_authoritative_history() {
    assert_history_required(Position::default());
    assert_history_required(Rotation::default());
    assert_history_required(LinearVelocity::ZERO);
    assert_history_required(AngularVelocity::ZERO);
    assert_history_required(ArenaBoundary {
        radius: 1200.0,
        target_radius: 1200.0,
    });
    assert_history_required(PlayerBlasters::default());
    assert_history_required(BlasterTrigger::default());
    assert_history_required(BlasterReload::default());
    assert_history_required(BlasterShot {
        position: Vec2::ZERO,
        direction: Vec2::X,
        ticks_left: 60,
    });
    assert_history_required(PlayerBoost::default());
    assert_history_required(PlayerHealth::default());
    assert_history_required(PlayerPhaseBeam::default());
}
