use bevy::prelude::*;
use bevy_resvg::prelude::SvgFile;

use super::chunks::BackdropChunk;
use super::layers::BACKDROP_LAYERS;

// Tile handles per layer, in the order the layer lists them.
#[derive(Resource)]
pub(super) struct BackdropTiles(pub(super) Vec<Vec<Handle<SvgFile>>>);

pub(super) fn load_backdrop_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut tiles = Vec::with_capacity(BACKDROP_LAYERS.len());
    for layer in &BACKDROP_LAYERS {
        let mut layer_tiles = Vec::with_capacity(layer.sprites.len());

        for path in layer.sprites {
            layer_tiles.push(asset_server.load(*path));
        }

        tiles.push(layer_tiles);
    }

    commands.insert_resource(BackdropTiles(tiles));
}

// `SvgPlugin` inserts the `Sprite` once the raster is ready, which is after the chunk is spawned,
// so size and mirroring are applied where the sprite appears instead of at spawn.
pub(super) fn shape_backdrop_chunks(mut chunks: Query<(&BackdropChunk, &mut Sprite)>) {
    for (chunk, mut sprite) in &mut chunks {
        let Some(layer) = BACKDROP_LAYERS.get(chunk.layer) else {
            continue;
        };

        sprite.custom_size = Some(Vec2::splat(layer.chunk));
        sprite.flip_x = chunk.flip_x;
        sprite.flip_y = chunk.flip_y;
    }
}
