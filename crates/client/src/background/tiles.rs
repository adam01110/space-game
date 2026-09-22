use bevy::prelude::*;

use super::chunks::BackdropChunk;
use super::layers::BACKDROP_LAYERS;

// Tile handles per layer, in the order the layer lists them.
#[derive(Resource)]
pub(super) struct BackdropTiles(pub(super) Vec<Vec<Handle<Image>>>);

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

// A chunk is spawned with a bare sprite, so the size and the mirroring of its cell are applied
// here. The pass runs every frame, which also covers a chunk spawned after it in the same frame.
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
