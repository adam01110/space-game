use bevy::prelude::*;

use project_protocol::ArenaBoundary;

use crate::{ARENA_RESIZE_SPEED, PlayerBundle, SERVER_UPS, arena_radius};

use crate::tests::support::arena_simulation;

#[test]
fn joins_grow_the_physical_radius_gradually() {
    let mut app = arena_simulation();
    for index in 0..4_u16 {
        app.world_mut()
            .spawn(PlayerBundle::new(Vec2::new(f32::from(index) * 80.0, 0.0)));
    }
    let mut previous = arena_radius(0);
    for tick in 0..600 {
        app.update();
        let arena = *app
            .world_mut()
            .query::<&ArenaBoundary>()
            .single(app.world())
            .expect("arena");
        assert_eq!(arena.target_radius, arena_radius(4));
        assert!(arena.radius >= previous);
        assert!(arena.radius - previous <= ARENA_RESIZE_SPEED / SERVER_UPS as f32 + 0.001);
        assert!(arena.radius <= arena.target_radius);
        if tick == 0 {
            assert!(arena.radius > previous);
            assert!(arena.radius < arena.target_radius);
        }
        previous = arena.radius;
    }
    assert_eq!(previous, arena_radius(4));
}
