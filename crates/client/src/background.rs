mod chunks;
mod layers;
mod stream;
mod tiles;

use bevy::{camera::CameraUpdateSystems, prelude::*, transform::TransformSystems};

use crate::camera::CameraSystems;
use crate::palette::Palette;

#[cfg(feature = "dev")]
pub(crate) use chunks::draw_chunk_outlines;
#[cfg(test)]
pub(crate) use chunks::{
    BackdropChunk, CHUNK_MARGIN, ChunkVariation, chunk_range, chunk_translation, chunk_variation,
};
#[cfg(test)]
pub(crate) use layers::BACKDROP_LAYERS;
pub(crate) use stream::BackdropDrift;
#[cfg(test)]
pub(crate) use stream::{BACKDROP_DRIFT, world_view};
use stream::{BackdropChunks, drift_backdrop, layout_backdrop};
use tiles::{load_backdrop_tiles, shape_backdrop_chunks};

pub(super) struct ClientBackgroundPlugin;

impl Plugin for ClientBackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Palette::PlumBlack.color()))
            .init_resource::<BackdropChunks>()
            .init_resource::<BackdropDrift>()
            .add_systems(Startup, load_backdrop_tiles)
            .add_systems(
                PostUpdate,
                (
                    drift_backdrop,
                    layout_backdrop
                        .after(CameraSystems::Follow)
                        .after(CameraUpdateSystems),
                    shape_backdrop_chunks,
                )
                    .chain()
                    .before(TransformSystems::Propagate),
            );
        #[cfg(feature = "dev")]
        app.add_systems(
            PostUpdate,
            draw_chunk_outlines
                .after(shape_backdrop_chunks)
                .before(TransformSystems::Propagate),
        );
    }
}
