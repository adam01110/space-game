use space_game_protocol::ArenaBoundary;

use crate::{
    ARENA_GROWTH_MULTIPLIER, ARENA_RESIZE_SPEED, BASE_ARENA_RADIUS, advance_arena, arena_radius,
};

#[test]
fn resizing_is_rate_limited_reversible_and_stops_at_target() {
    let mut arena = ArenaBoundary::new(BASE_ARENA_RADIUS);
    arena.target_radius = arena_radius(4);
    advance_arena(&mut arena, 0.25);
    assert_eq!(arena.radius, BASE_ARENA_RADIUS + ARENA_RESIZE_SPEED * 0.25);
    arena.target_radius = BASE_ARENA_RADIUS;
    advance_arena(&mut arena, 0.125);
    assert_eq!(arena.radius, BASE_ARENA_RADIUS + ARENA_RESIZE_SPEED * 0.125);
    advance_arena(&mut arena, 1.0);
    assert_eq!(arena.radius, BASE_ARENA_RADIUS);
    arena.target_radius = BASE_ARENA_RADIUS + 1.0;
    advance_arena(&mut arena, 1.0);
    assert_eq!(arena.radius, arena.target_radius);
}

#[test]
fn radius_scales_population_growth_without_changing_solo_size() {
    assert_eq!(arena_radius(0), BASE_ARENA_RADIUS);
    assert_eq!(arena_radius(1), BASE_ARENA_RADIUS);
    assert_eq!(
        arena_radius(4),
        BASE_ARENA_RADIUS * (1.0 + ARENA_GROWTH_MULTIPLIER)
    );
    assert_eq!(
        arena_radius(9),
        BASE_ARENA_RADIUS * (1.0 + 2.0 * ARENA_GROWTH_MULTIPLIER)
    );
}
