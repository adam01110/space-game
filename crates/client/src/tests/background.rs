use std::time::Duration;

use bevy::{
    asset::AssetPlugin,
    image::{CompressedImageFormats, ImageLoader, ImagePlugin},
    prelude::*,
    time::TimeUpdateStrategy,
    window::{PrimaryWindow, WindowResolution},
};

use crate::{
    background::{
        BACKDROP_DRIFT, BACKDROP_LAYERS, BackdropChunk, BackdropDrift, ClientBackgroundPlugin,
        chunk_range, chunk_translation, chunk_variation, world_view,
    },
    camera::{GameplayCamera, PIXEL_SIZE},
};

const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

// Starts a client with the backdrop plugin, a window and a gameplay camera, but no render app:
// the layout runs against the same data as in the game.
fn backdrop_app() -> App {
    let mut app = App::new();
    app.insert_resource(crate::network::GuestConnection {
        #[cfg(not(target_family = "wasm"))]
        started_by_play: true,
        ..default()
    });
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        ImagePlugin::default_nearest(),
        ClientBackgroundPlugin,
    ))
    // The tiles are ordinary `Image` assets, whose loader the renderer registers while
    // `ImagePlugin` only reserves it, so an app without a renderer registers it itself.
    .register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));
    app.world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .insert(Window {
            resolution: WindowResolution::new(1280, 720),
            ..default()
        });
    app.world_mut().spawn((
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: PIXEL_SIZE,
            ..OrthographicProjection::default_2d()
        }),
        GameplayCamera,
    ));

    // The layout tests pin where chunks land, so the clock is fixed: time only moves when a test
    // steps it.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));

    app
}

fn drift(app: &App) -> Vec2 {
    app.world().resource::<BackdropDrift>().offset
}

fn move_camera(app: &mut App, position: Vec2) {
    let mut query = app
        .world_mut()
        .query_filtered::<&mut Transform, With<GameplayCamera>>();
    if let Ok(mut transform) = query.single_mut(app.world_mut()) {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}

fn chunks(app: &mut App) -> Vec<(usize, Vec2)> {
    let mut query = app.world_mut().query::<(&BackdropChunk, &Transform)>();
    query
        .iter(app.world())
        .map(|(chunk, transform)| (chunk.layer, transform.translation.xy()))
        .collect()
}

// Every layer has to hold exactly the cells that cover the view around `position`, and its tiles
// have to reach past the edges of that view. Chunks are placed in the world, but a layer streams
// its grid in its own space, so the parallax offset and drift are removed before comparing.
fn assert_covered(app: &mut App, position: Vec2) {
    let found = chunks(app);
    let travelled = drift(app);

    for (index, layer) in BACKDROP_LAYERS.into_iter().enumerate() {
        let layer_position = (position - travelled) * layer.parallax;
        let anchor = position - layer_position;
        let range = chunk_range(layer_position, WINDOW, layer.chunk);
        let centres: Vec<Vec2> = found
            .iter()
            .filter(|(chunk_layer, _)| *chunk_layer == index)
            .map(|(_, centre)| *centre - anchor)
            .collect();
        let expected =
            usize::try_from((range.max.x - range.min.x + 1) * (range.max.y - range.min.y + 1))
                .unwrap_or(0);

        assert_eq!(centres.len(), expected, "layer {index} chunk count");

        let half = Vec2::splat(layer.chunk * 0.5);
        let min = centres.iter().copied().reduce(Vec2::min).expect("chunks") - half;
        let max = centres.iter().copied().reduce(Vec2::max).expect("chunks") + half;

        assert!(
            min.x <= layer_position.x - WINDOW.x * 0.5,
            "layer {index} left {min:?}"
        );
        assert!(
            min.y <= layer_position.y - WINDOW.y * 0.5,
            "layer {index} below {min:?}"
        );
        assert!(
            max.x >= layer_position.x + WINDOW.x * 0.5,
            "layer {index} right {max:?}"
        );
        assert!(
            max.y >= layer_position.y + WINDOW.y * 0.5,
            "layer {index} above {max:?}"
        );
    }
}

// Chunk centres per layer, ordered, so the same chunk can be followed across a camera move.
fn centres_by_layer(app: &mut App) -> Vec<Vec<Vec2>> {
    let found = chunks(app);
    BACKDROP_LAYERS
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut centres: Vec<Vec2> = found
                .iter()
                .filter(|(chunk_layer, _)| *chunk_layer == index)
                .map(|(_, centre)| *centre)
                .collect();
            centres
                .sort_by(|left, right| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)));
            centres
        })
        .collect()
}

#[test]
fn a_chunk_sits_on_the_centre_of_its_cell_of_the_grid() {
    let layer = BACKDROP_LAYERS[0];
    let half = Vec2::splat(layer.chunk * 0.5);

    assert_eq!(
        chunk_translation(layer, IVec2::ZERO),
        Vec3::new(half.x, half.y, layer.z)
    );
    assert_eq!(
        chunk_translation(layer, IVec2::new(-2, 3)).xy(),
        Vec2::new(-2.0 * layer.chunk, 3.0 * layer.chunk) + half
    );
}

#[test]
fn a_chunk_grid_of_every_layer_covers_the_view_around_the_camera() {
    // Near the arena, at a far corner of it, and far outside anything the arena defines.
    let positions = [
        Vec2::ZERO,
        Vec2::new(1200.0, -900.0),
        Vec2::new(-48_000.0, 37_000.0),
        Vec2::new(7.5, -3.25),
    ];

    for layer in BACKDROP_LAYERS {
        for position in positions {
            let range = chunk_range(position, WINDOW, layer.chunk);
            let covered_min = range.min.as_vec2() * layer.chunk;
            let covered_max = (range.max.as_vec2() + Vec2::ONE) * layer.chunk;

            // The grid starts outside the view and still reaches past its far edge.
            assert!(covered_min.x <= position.x - WINDOW.x * 0.5, "{position:?}");
            assert!(covered_min.y <= position.y - WINDOW.y * 0.5, "{position:?}");
            assert!(covered_max.x >= position.x + WINDOW.x * 0.5, "{position:?}");
            assert!(covered_max.y >= position.y + WINDOW.y * 0.5, "{position:?}");
        }
    }
}

#[test]
fn view_falls_back_to_the_window_until_the_camera_measures_its_canvas() {
    let window = Window {
        resolution: WindowResolution::new(1280, 720),
        ..default()
    };
    let unmeasured = Projection::Orthographic(OrthographicProjection {
        scale: PIXEL_SIZE,
        ..OrthographicProjection::default_2d()
    });
    let measured = Projection::Orthographic(OrthographicProjection {
        scale: PIXEL_SIZE,
        area: Rect::new(-5120.0, -2880.0, 5120.0, 2880.0),
        ..OrthographicProjection::default_2d()
    });

    assert_eq!(world_view(&unmeasured, &window), WINDOW);
    assert_eq!(world_view(&measured, &window), Vec2::new(10_240.0, 5760.0));
    // A window that is not a whole number of canvas pixels is rounded up, as the canvas is.
    assert_eq!(
        world_view(
            &unmeasured,
            &Window {
                resolution: WindowResolution::new(1279, 719),
                ..default()
            }
        ),
        WINDOW
    );
}

#[test]
fn chunk_variation_is_stable_and_spread_over_the_tile_set() {
    let tiles = BACKDROP_LAYERS[0].sprites.len();
    assert_eq!(
        chunk_variation(1, IVec2::new(4, -7), tiles),
        chunk_variation(1, IVec2::new(4, -7), tiles)
    );
    assert_ne!(
        chunk_variation(1, IVec2::new(4, -7), tiles),
        chunk_variation(1, IVec2::new(4, -6), tiles)
    );

    let mut seen: Vec<crate::background::ChunkVariation> = Vec::new();
    for x in 0..6 {
        for y in 0..6 {
            let variation = chunk_variation(1, IVec2::new(x, y), tiles);
            if !seen.contains(&variation) {
                seen.push(variation);
            }
        }
    }

    // Three tiles, two mirrorings and four quarter turns are twenty-four treatments; a small
    // patch of the grid has to use most of them.
    let mut variants: Vec<usize> = seen.iter().map(|entry| entry.tile).collect();
    variants.sort_unstable();
    variants.dedup();

    assert_eq!(variants.len(), tiles, "{seen:?}");
    assert!(seen.len() >= 12, "{seen:?}");
}

// A layer of one tile draws its variety from the chunk's mirroring and quarter turn, so
// neighbouring cells still have to look different.
#[test]
fn a_layer_of_one_tile_still_varies_how_its_chunks_are_treated() {
    for (index, layer) in BACKDROP_LAYERS.iter().enumerate() {
        if layer.sprites.len() != 1 {
            continue;
        }

        let mut seen: Vec<crate::background::ChunkVariation> = Vec::new();
        for x in 0..6 {
            for y in 0..6 {
                let variation = chunk_variation(index, IVec2::new(x, y), layer.sprites.len());
                assert_eq!(variation.tile, 0, "layer {index} picked a missing tile");
                if !seen.contains(&variation) {
                    seen.push(variation);
                }
            }
        }

        // Two mirrorings and four quarter turns are eight treatments at least.
        assert!(seen.len() >= 8, "layer {index} repeats itself: {seen:?}");
    }
}

#[test]
fn backdrop_rasters_match_their_chunk_size() {
    for layer in BACKDROP_LAYERS {
        // One texel per canvas pixel.
        let raster = Vec2::splat(layer.chunk / PIXEL_SIZE);

        for path in layer.sprites {
            let file = format!("{}/../../assets/{path}", env!("CARGO_MANIFEST_DIR"));
            let source = std::fs::read(&file).expect("backdrop tile");

            assert_eq!(png_size(&source), raster, "{file}");
        }
    }
}

// Width and height out of the PNG header: the format pins its first chunk to IHDR, whose two
// big-endian dimensions sit right after the chunk name.
fn png_size(source: &[u8]) -> Vec2 {
    const SIGNATURE: [u8; 8] = [137, b'P', b'N', b'G', 13, 10, 26, 10];

    assert_eq!(source.get(..SIGNATURE.len()), Some(SIGNATURE.as_slice()));
    assert_eq!(source.get(12..16), Some(b"IHDR".as_slice()));

    let dimension = |at: usize| {
        let bytes = source
            .get(at..at + 4)
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .expect("png dimension");

        f32::from(u16::try_from(u32::from_be_bytes(bytes)).expect("tile dimension"))
    };

    Vec2::new(dimension(16), dimension(20))
}

// The backdrop carries its own travel, so the layers keep sliding while the camera holds still.
// The travel is shared and every layer scales it by its parallax, so the nearest layer slides
// furthest.
#[test]
fn the_backdrop_slides_in_one_direction_scaled_by_its_parallax() {
    let mut app = backdrop_app();
    for _ in 0..3 {
        app.update();
    }

    let step = Duration::from_millis(200);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let before = centres_by_layer(&mut app);
    app.update();
    let after = centres_by_layer(&mut app);

    let seconds = step.as_secs_f32();
    let mut on_screen: Vec<Vec2> = Vec::new();
    for (index, layer) in BACKDROP_LAYERS.into_iter().enumerate() {
        // The step is far shorter than every chunk, so no cell enters or leaves the view and the
        // ordered centre lists line up one to one.
        assert_eq!(
            before[index].len(),
            after[index].len(),
            "layer {index} streamed"
        );

        let expected = BACKDROP_DRIFT * seconds * layer.parallax;
        for (start, end) in before[index].iter().zip(&after[index]) {
            let slid = *end - *start;
            assert!(
                (slid - expected).length() < 0.001,
                "layer {index} slid {slid:?}, expected {expected:?}"
            );
        }

        on_screen.push(expected);
    }

    // Every layer slides the same way, so the backdrop reads as one motion, and the nearer the
    // layer the further it slides.
    for pair in on_screen.windows(2) {
        assert!(
            pair[0].length() < pair[1].length(),
            "layers out of order: {on_screen:?}"
        );
        assert!(
            pair[0].normalize().dot(pair[1].normalize()) > 0.999,
            "layers drift apart: {on_screen:?}"
        );
    }
}

// The layers have to separate visually: each trails the camera by its own factor instead of
// travelling with the world, and the nearer the layer the more of the camera travel it follows.
#[test]
fn every_layer_trails_the_camera_by_its_parallax_factor() {
    let mut app = backdrop_app();
    for _ in 0..3 {
        app.update();
    }

    let before = centres_by_layer(&mut app);
    let travel = Vec2::new(200.0, -120.0);
    move_camera(&mut app, travel);
    app.update();
    let after = centres_by_layer(&mut app);

    // Chunks are placed in the world, where the offset is `(1 - parallax) * travel`. On screen the
    // camera travel is subtracted again, so a layer appears to slide by `-parallax * travel`.
    let mut on_screen: Vec<Vec2> = Vec::new();
    for (index, layer) in BACKDROP_LAYERS.into_iter().enumerate() {
        // A move of 200 units is short of every chunk size, so no cell enters or leaves the view
        // and the ordered centre lists line up one to one.
        assert_eq!(
            before[index].len(),
            after[index].len(),
            "layer {index} streamed"
        );

        let expected = travel * (1.0 - layer.parallax);
        for (start, end) in before[index].iter().zip(&after[index]) {
            let moved = *end - *start;
            assert!(
                (moved - expected).length() < 0.01,
                "layer {index} moved {moved:?}, expected {expected:?}"
            );
        }

        on_screen.push(expected - travel);
    }

    // The nearer the layer, the further it slides past the camera, which is what reads as parallax.
    for pair in on_screen.windows(2) {
        assert!(
            pair[0].length() < pair[1].length(),
            "layers out of order: {on_screen:?}"
        );
    }
}

// The depth order is carried by the layer list itself: back to front, each layer is drawn nearer
// and scrolls faster. Chunk sizes deliberately do not follow the order, so the grids never line up
// with each other.
#[test]
fn the_layers_are_ordered_from_back_to_front() {
    for pair in BACKDROP_LAYERS.windows(2) {
        let (back, front) = (pair[0], pair[1]);

        assert!(back.z < front.z, "z out of order: {back:?} {front:?}");
        assert!(
            back.parallax < front.parallax,
            "parallax out of order: {back:?} {front:?}"
        );
        assert_ne!(back.chunk, front.chunk, "grids line up: {back:?} {front:?}");
        assert!(!back.sprites.is_empty(), "layer without a tile: {back:?}");
        assert!(!front.sprites.is_empty(), "layer without a tile: {front:?}");
    }
}

fn loaded_tiles(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<&Sprite, With<BackdropChunk>>();
    let images = app.world().resource::<Assets<Image>>();

    query
        .iter(app.world())
        .filter(|sprite| images.contains(&sprite.image))
        .count()
}

#[test]
fn travelling_spawns_only_the_chunks_that_enter_the_view() {
    let mut app = backdrop_app();
    for _ in 0..3 {
        app.update();
    }
    assert_covered(&mut app, Vec2::ZERO);

    // A trip across and far beyond the arena, one chunk border at a time.
    for step in 1..=40 {
        let travelled = Vec2::new(3000.0, -1500.0) * f32::from(u16::try_from(step).unwrap_or(0));
        move_camera(&mut app, travelled);
        app.update();
        assert_covered(&mut app, travelled);
    }

    // Chunks left behind are gone. Rasters load asynchronously, so wait for them before checking
    // that every live chunk is painted at its chunk size, mirrored as its cell asks for.
    let mut waited = 0;
    while loaded_tiles(&mut app) < chunks(&mut app).len() {
        waited += 1;
        assert!(waited < 2_000, "tile rasters did not load");
        app.update();
    }

    let mut query = app.world_mut().query::<(&BackdropChunk, &Sprite)>();
    let mut painted = 0;
    for (chunk, sprite) in query.iter(app.world()) {
        let layer = BACKDROP_LAYERS.get(chunk.layer).expect("known layer");
        assert_eq!(sprite.custom_size, Some(Vec2::splat(layer.chunk)));
        assert_eq!(sprite.flip_x, chunk.flip_x);
        assert_eq!(sprite.flip_y, chunk.flip_y);
        painted += 1;
    }

    assert_eq!(painted, chunks(&mut app).len(), "every chunk has a sprite");
}
