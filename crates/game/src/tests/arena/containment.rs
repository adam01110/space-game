use bevy::prelude::*;

use crate::contain_circle;

#[test]
fn contains_full_body_at_every_angle_even_after_large_steps() {
    for angle in 0..360_u16 {
        let normal = Vec2::from_angle(f32::from(angle).to_radians());
        let mut position = normal * 100_000.0;
        let mut velocity = normal * 20_000.0;
        contain_circle(768.0, 23.4, &mut position, &mut velocity);
        assert!(position.length() + 23.4 <= 768.001);
        assert!(velocity.length() < 0.01);
    }
}

#[test]
fn shrink_preserves_tangential_and_inward_motion() {
    let mut position = Vec2::new(2000.0, 0.0);
    let mut velocity = Vec2::new(-20.0, 50.0);
    contain_circle(768.0, 23.4, &mut position, &mut velocity);
    assert_eq!(position, Vec2::new(744.6, 0.0));
    assert_eq!(velocity, Vec2::new(-20.0, 50.0));
    velocity = Vec2::new(20.0, 50.0);
    position.x = 800.0;
    contain_circle(768.0, 23.4, &mut position, &mut velocity);
    assert_eq!(velocity, Vec2::new(0.0, 50.0));
}

#[test]
fn interior_motion_is_unchanged() {
    let mut position = Vec2::ZERO;
    let mut velocity = Vec2::new(100.0, 200.0);
    contain_circle(768.0, 23.4, &mut position, &mut velocity);
    assert_eq!(position, Vec2::ZERO);
    assert_eq!(velocity, Vec2::new(100.0, 200.0));
}
