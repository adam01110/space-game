use bevy::prelude::*;
use lightyear::prelude::Predicted;

use project_game::phase_beam::{BEAM_LENGTH, BEAM_WIDTH, NOSE_OFFSET, PhaseBeamSegment};
use project_protocol::Player;

#[derive(Component)]
pub(super) struct BeamVisual;

type NeedsBeamVisual = (
    With<Player>,
    With<Predicted>,
    With<Sprite>,
    Without<BeamVisual>,
);

pub(super) fn add_phase_beam_visuals(
    mut commands: Commands,
    players: Query<Entity, NeedsBeamVisual>,
) {
    for player in &players {
        // Parenting follows the rendered ship pose, including prediction smoothing,
        // and despawns the beam with its ship.
        commands.spawn((
            Sprite::from_color(
                Color::srgb(0.3, 1.0, 1.0),
                Vec2::new(BEAM_WIDTH, BEAM_LENGTH),
            ),
            Transform::from_xyz(0.0, NOSE_OFFSET + BEAM_LENGTH / 2.0, 1.0),
            Visibility::Hidden,
            ChildOf(player),
            BeamVisual,
        ));
    }
}

// The predicted segment reflects both held input and available charge. The child sprite owns
// its Visibility; toggling the parent would hide the ship subtree instead.
type BeamVisualQuery<'w, 's> =
    Query<'w, 's, (&'static ChildOf, &'static mut Visibility), With<BeamVisual>>;

pub(super) fn update_phase_beam_visuals(
    beams: BeamVisualQuery,
    active_beams: Query<(), With<PhaseBeamSegment>>,
) {
    for (parent, mut visibility) in beams {
        *visibility = match active_beams.contains(parent.0) {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        };
    }
}
