use bevy::prelude::Vec2;
use bevy_resvg::resvg::usvg::{Options, Tree};

use space_game_protocol::{PlayerBoost, PlayerInput};

use crate::flame::{FlameColor, flame_color};

#[test]
fn flame_sprites_are_valid_static_svgs() {
    for source in [
        include_str!("../../../../assets/sprites/flame-red-0.svg"),
        include_str!("../../../../assets/sprites/flame-red-1.svg"),
        include_str!("../../../../assets/sprites/flame-red-2.svg"),
        include_str!("../../../../assets/sprites/flame-blue-0.svg"),
        include_str!("../../../../assets/sprites/flame-blue-1.svg"),
        include_str!("../../../../assets/sprites/flame-blue-2.svg"),
    ] {
        Tree::from_data(source.as_bytes(), &Options::default()).expect("valid flame sprite");
    }
}

#[test]
fn flame_tracks_movement_towards_cursor_and_boost_takes_priority() {
    let mut input = PlayerInput::default();
    let mut boost = PlayerBoost::default();

    assert_eq!(flame_color(&input, &boost), None);
    input.aim = Vec2::X;
    input.movement = Vec2::X;
    assert_eq!(flame_color(&input, &boost), Some(FlameColor::Red));

    input.movement = Vec2::Y;
    assert_eq!(flame_color(&input, &boost), None);
    input.movement = Vec2::NEG_X;
    assert_eq!(flame_color(&input, &boost), None);

    input.boost = true;
    assert_eq!(flame_color(&input, &boost), Some(FlameColor::Blue));
    input.movement = Vec2::X;
    assert_eq!(flame_color(&input, &boost), Some(FlameColor::Blue));

    boost.0.drain(2, std::time::Duration::from_secs(25));
    assert_eq!(flame_color(&input, &boost), Some(FlameColor::Red));
    input.movement = Vec2::ZERO;
    assert_eq!(flame_color(&input, &boost), None);
}
