// The debug zoom keybind only exists in development builds, so its test does too.
#![cfg(feature = "dev")]

use bevy::{ecs::query::QueryFilter, prelude::*};

use crate::camera::{
    toggle_debug_zoom, CanvasCamera, DebugCamera, DebugZoom, GameplayCamera, CANVAS_CAMERA_SCALE,
    DEBUG_ZOOM_FACTOR, PIXEL_SIZE,
};

fn orthographic(scale: f32) -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scale,
        ..OrthographicProjection::default_2d()
    })
}

fn zoom_app() -> App {
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<DebugZoom>()
        .add_systems(Update, toggle_debug_zoom);
    app.world_mut()
        .spawn((Camera2d, orthographic(PIXEL_SIZE), GameplayCamera));
    app.world_mut()
        .spawn((Camera2d, orthographic(CANVAS_CAMERA_SCALE), CanvasCamera));
    app.world_mut()
        .spawn((Camera2d, orthographic(1.0), DebugCamera));

    app
}

fn scale<F: QueryFilter>(app: &mut App) -> f32 {
    let mut query = app.world_mut().query_filtered::<&Projection, F>();
    let projection = query.single(app.world()).expect("camera");
    let Projection::Orthographic(orthographic) = projection else {
        panic!("expected an orthographic camera");
    };

    orthographic.scale
}

// Every scale this test pins comes out of exact binary values, so the comparison is only a
// tolerance to keep the equality off the float lints.
fn assert_scale(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected scale {expected}, got {actual}"
    );
}

fn press_zoom(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyZ);
}

fn release_zoom(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyZ);
}

// Bevy's input systems clear the press and release edges every frame; the app here runs without
// them, so the test advances the keyboard by hand: the frame runs on the edges it was given, then
// forgets them, which leaves a held key pressed but off its press edge.
fn next_frame(app: &mut App) {
    app.world_mut().run_schedule(Update);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

// The gameplay camera stays put because the backdrop streamer streams the view of its projection:
// moving it would stream a wider view and hide the edge of the backdrop this keybind exists to
// show. Only the canvas display and the diagnostics drawn to the window shrink.
#[test]
fn the_debug_zoom_key_shrinks_the_display_without_moving_the_streamed_view() {
    let mut app = zoom_app();
    next_frame(&mut app);
    assert_scale(scale::<With<CanvasCamera>>(&mut app), CANVAS_CAMERA_SCALE);

    press_zoom(&mut app);
    next_frame(&mut app);
    assert_scale(
        scale::<With<CanvasCamera>>(&mut app),
        CANVAS_CAMERA_SCALE * DEBUG_ZOOM_FACTOR,
    );
    assert_scale(scale::<With<DebugCamera>>(&mut app), DEBUG_ZOOM_FACTOR);
    assert_scale(scale::<With<GameplayCamera>>(&mut app), PIXEL_SIZE);

    // The key is a toggle on the press edge, so holding it does not flip the zoom back.
    next_frame(&mut app);
    assert_scale(
        scale::<With<CanvasCamera>>(&mut app),
        CANVAS_CAMERA_SCALE * DEBUG_ZOOM_FACTOR,
    );

    release_zoom(&mut app);
    next_frame(&mut app);
    assert_scale(
        scale::<With<CanvasCamera>>(&mut app),
        CANVAS_CAMERA_SCALE * DEBUG_ZOOM_FACTOR,
    );
    assert_scale(scale::<With<GameplayCamera>>(&mut app), PIXEL_SIZE);

    press_zoom(&mut app);
    next_frame(&mut app);
    assert_scale(scale::<With<CanvasCamera>>(&mut app), CANVAS_CAMERA_SCALE);
    assert_scale(scale::<With<DebugCamera>>(&mut app), 1.0);
    assert_scale(scale::<With<GameplayCamera>>(&mut app), PIXEL_SIZE);
}
