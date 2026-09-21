#[cfg(feature = "dev")]
use avian2d::prelude::PhysicsGizmos;
#[cfg(feature = "dev")]
use bevy::prelude::*;
#[cfg(feature = "dev")]
use lightyear::prelude::Predicted;

#[cfg(feature = "dev")]
use space_game_protocol::ArenaBoundary;

#[cfg(feature = "dev")]
use crate::palette::Palette;

// Presentation is deliberately separate from the gameplay constraint. Reuse the
// existing native-resolution debug layer without changing any camera/MSAA settings.
// Only compiled into the physics debug build.
#[cfg(feature = "dev")]
pub(super) fn draw_arena(
    arenas: Query<&ArenaBoundary, With<Predicted>>,
    mut gizmos: Gizmos<PhysicsGizmos>,
) {
    if let Ok(arena) = arenas.single() {
        gizmos
            .circle_2d(Vec2::ZERO, arena.radius, Palette::Olive.color())
            .resolution(512);
    }
}
