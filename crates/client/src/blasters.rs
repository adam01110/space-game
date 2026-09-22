use bevy::prelude::*;
use space_game_protocol::{BlasterShot, BlasterTrajectory};

use crate::palette::Palette;

fn shot_transform(shot: &BlasterShot, trajectory: BlasterTrajectory) -> Transform {
    Transform::from_translation(shot.position.extend(1.0)).with_rotation(Quat::from_rotation_z(
        trajectory.direction.y.atan2(trajectory.direction.x) - std::f32::consts::FRAC_PI_2,
    ))
}

pub(super) fn add_shot_visuals(
    mut commands: Commands,
    shots: Query<(Entity, &BlasterShot, &BlasterTrajectory), Without<Sprite>>,
) {
    for (entity, shot, trajectory) in &shots {
        commands.entity(entity).insert((
            Sprite::from_color(Palette::Citron.color(), Vec2::new(4.0, 16.0)),
            shot_transform(shot, *trajectory),
        ));
    }
}

pub(super) fn update_shot_visuals(
    mut shots: Query<(&BlasterShot, &BlasterTrajectory, &mut Transform)>,
) {
    for (shot, trajectory, mut transform) in &mut shots {
        *transform = shot_transform(shot, *trajectory);
    }
}
