use avian2d::prelude::PhysicsGizmos;
use bevy::prelude::*;
use lightyear::prelude::Predicted;

use project_protocol::ArenaBoundary;

/*
Presentation is deliberately separate from the gameplay constraint. Reuse the
existing native-resolution debug layer without changing any camera/MSAA settings.
*/
pub(super) fn draw_arena(
    arenas: Query<&ArenaBoundary, With<Predicted>>,
    mut gizmos: Gizmos<PhysicsGizmos>,
) {
    if let Ok(arena) = arenas.single() {
        gizmos
            .circle_2d(Vec2::ZERO, arena.radius, Color::srgb(1.0, 0.75, 0.2))
            .resolution(512);
    }
}
