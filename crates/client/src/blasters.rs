use bevy::prelude::*;
use space_game_protocol::BlasterShot;

fn shot_transform(shot: &BlasterShot) -> Transform {
    Transform::from_translation(shot.position.extend(1.0)).with_rotation(Quat::from_rotation_z(
        shot.direction.y.atan2(shot.direction.x) - std::f32::consts::FRAC_PI_2,
    ))
}

pub(super) fn add_shot_visuals(
    mut commands: Commands,
    shots: Query<(Entity, &BlasterShot), Without<Sprite>>,
) {
    for (entity, shot) in &shots {
        commands.entity(entity).insert((
            Sprite::from_color(Color::srgb(1.0, 0.9, 0.3), Vec2::new(4.0, 16.0)),
            shot_transform(shot),
        ));
    }
}

pub(super) fn update_shot_visuals(mut shots: Query<(&BlasterShot, &mut Transform)>) {
    for (shot, mut transform) in &mut shots {
        *transform = shot_transform(shot);
    }
}
