use avian2d::prelude::{Position, Rotation};
use bevy::prelude::*;
use bevy_resvg::prelude::{Svg, SvgFile};
use lightyear::prelude::Predicted;

use space_game_protocol::Player;

use crate::{
    flame::FlameAssets,
    player::{PlayerSprite, add_player_visuals},
};

#[test]
fn player_visuals_wait_for_the_svg_and_reuse_its_handle() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<SvgFile>()
        .init_resource::<FlameAssets>()
        .add_systems(Update, add_player_visuals);
    let handle = app.world().resource::<Assets<SvgFile>>().reserve_handle();
    app.insert_resource(PlayerSprite(handle.clone()));
    let first = app
        .world_mut()
        .spawn((Player, Predicted, Position::default(), Rotation::default()))
        .id();

    app.update();
    assert!(app.world().get::<Svg>(first).is_none());

    app.world_mut()
        .resource_mut::<Assets<SvgFile>>()
        .insert(&handle, SvgFile(Image::default()))
        .expect("reserved asset handle");
    app.update();
    assert_eq!(app.world().get::<Svg>(first).expect("player SVG").0, handle);

    let second = app
        .world_mut()
        .spawn((Player, Predicted, Position::default(), Rotation::default()))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Svg>(second).expect("player SVG").0,
        handle
    );
}
