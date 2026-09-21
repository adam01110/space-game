use std::f32::consts::FRAC_PI_2;

use bevy::{platform::collections::HashMap, prelude::*, window::PrimaryWindow};
use bevy_resvg::prelude::Svg;

use crate::camera::{GameplayCamera, PIXEL_SIZE};

use super::chunks::{BackdropChunk, CHUNK_MARGIN, chunk_range, chunk_translation, chunk_variation};
use super::layers::{BACKDROP_LAYERS, BackdropLayer};
use super::tiles::BackdropTiles;

// Travel the backdrop carries on its own, in world units per second. Each layer scales it by its
// parallax, so the depths separate here exactly as they do when the camera flies.
pub(crate) const BACKDROP_DRIFT: Vec2 = Vec2::new(-8.0, 4.0);

// How far the backdrop has travelled on its own. Held in the space a camera position lives in,
// rather than in a layer's, so every layer scales the one offset by its parallax.
#[derive(Resource, Default)]
pub(crate) struct BackdropDrift {
    pub(crate) offset: Vec2,
}

// The chunks that are currently spawned, so travel only spawns what entered the view and only
// despawns what left it.
#[derive(Resource, Default)]
pub(super) struct BackdropChunks {
    live: HashMap<(usize, IVec2), Entity>,
}

// The camera query names its filter, so it stays disjoint from the chunk query.
type CameraViewFilter = (With<GameplayCamera>, Without<BackdropChunk>);

pub(super) fn drift_backdrop(time: Res<Time>, mut drift: ResMut<BackdropDrift>) {
    drift.offset += BACKDROP_DRIFT * time.delta_secs();
}

// Keep the chunks covering the view around the camera, wherever it travels. Cells are remembered
// rather than recomputed from scratch, so crossing a chunk border spawns one row or column
// instead of rebuilding the backdrop.
pub(super) fn layout_backdrop(
    mut commands: Commands,
    camera: Single<(&Transform, &Projection), CameraViewFilter>,
    window: Single<&Window, With<PrimaryWindow>>,
    tiles: Res<BackdropTiles>,
    drift: Res<BackdropDrift>,
    mut chunks: ResMut<BackdropChunks>,
    mut placed: Query<&mut Transform, With<BackdropChunk>>,
) {
    let (camera_transform, projection) = *camera;
    let view = world_view(projection, *window);
    let position = camera_transform.translation.xy();
    let travelled = position - drift.offset;

    for (index, layer) in BACKDROP_LAYERS.into_iter().enumerate() {
        let layer_position = travelled * layer.parallax;
        LayerStream {
            index,
            layer,
            layer_position,
            anchor: (position - layer_position).extend(0.0),
            tiles: &tiles,
            chunks: &mut chunks.live,
        }
        .layout(view, commands.reborrow(), placed.reborrow());
    }
}

// The area the gameplay camera shows, in world units. The projection is authoritative, but it
// still holds its placeholder area until the camera has measured its canvas, so the size derived
// from the window is used as a floor.
pub(crate) fn world_view(projection: &Projection, window: &Window) -> Vec2 {
    let from_projection = match projection {
        Projection::Orthographic(orthographic) => orthographic.area.size(),
        _ => Vec2::ZERO,
    };

    let from_window = (Vec2::new(window.width(), window.height()) / PIXEL_SIZE).ceil() * PIXEL_SIZE;
    from_projection.max(from_window)
}

// One layer's streaming pass. A layer is streamed in its own space, as if the camera sat at
// `(position - drift) * parallax` inside it, and the difference back to the camera position is
// added to every chunk as a world `anchor` offset. That offset is what makes a layer trail the
// world, and the drift rides the same offset, so a layer slides by its parallax fraction of it
// too.
struct LayerStream<'a> {
    index: usize,
    layer: BackdropLayer,
    layer_position: Vec2,
    anchor: Vec3,
    tiles: &'a BackdropTiles,
    chunks: &'a mut HashMap<(usize, IVec2), Entity>,
}

impl LayerStream<'_> {
    // Retire the cells the layer position left, then spawn or re-place every cell the view around
    // it needs. Chunks are placed in the world, so re-placing them by the anchor offset is what
    // makes a layer trail the camera by `(1 - parallax) * camera travel`.
    fn layout(
        &mut self,
        view: Vec2,
        mut commands: Commands,
        mut placed: Query<&mut Transform, With<BackdropChunk>>,
    ) {
        let range = chunk_range(self.layer_position, view, self.layer.chunk, CHUNK_MARGIN);

        // Chunks the camera left behind.
        self.chunks.retain(|&(chunk_layer, cell), &mut entity| {
            let keep = chunk_layer != self.index || range.contains(cell);
            if !keep {
                debug!(layer = self.index, ?cell, "Retiring backdrop chunk");
                commands.entity(entity).try_despawn();
            }

            keep
        });

        for x in range.min.x..=range.max.x {
            for y in range.min.y..=range.max.y {
                self.place(IVec2::new(x, y), &mut commands, &mut placed);
            }
        }
    }

    // Bring one cell up to date: a live chunk only has to follow the anchor, everything else is
    // spawned once and stays fixed by its cell.
    fn place(
        &mut self,
        cell: IVec2,
        commands: &mut Commands,
        placed: &mut Query<&mut Transform, With<BackdropChunk>>,
    ) {
        let translation = chunk_translation(self.layer, cell) + self.anchor;

        if let Some(&entity) = self.chunks.get(&(self.index, cell)) {
            if let Ok(mut transform) = placed.get_mut(entity) {
                transform.translation = translation;
            }

            return;
        }

        let variation = chunk_variation(self.index, cell, self.layer.sprites.len());

        let Some(path) = self
            .tiles
            .0
            .get(self.index)
            .and_then(|set| set.get(variation.tile))
        else {
            return;
        };

        let entity = commands
            .spawn((
                Svg(path.clone()),
                BackdropChunk {
                    layer: self.index,
                    flip_x: variation.flip_x,
                    flip_y: variation.flip_y,
                },
                Transform::from_translation(translation)
                    .with_rotation(Quat::from_rotation_z(f32::from(variation.turn) * FRAC_PI_2)),
            ))
            .id();

        self.chunks.insert((self.index, cell), entity);
    }
}
