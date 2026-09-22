#[cfg(feature = "dev")]
use avian2d::prelude::PhysicsGizmos;
#[cfg(feature = "dev")]
use bevy::math::Rot2;
use bevy::prelude::*;

use super::layers::BackdropLayer;
#[cfg(feature = "dev")]
use super::layers::BACKDROP_LAYERS;
#[cfg(feature = "dev")]
use crate::palette::Palette;

// A live chunk of the backdrop, addressed by its cell in its layer's grid.
#[derive(Component)]
pub(crate) struct BackdropChunk {
    pub(crate) layer: usize,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
}

// Tile, mirroring and quarter turn for one chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChunkVariation {
    pub(crate) tile: usize,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
    pub(super) turn: u8,
}

// Centre of a cell in the layer's own space.
pub(crate) fn chunk_translation(layer: BackdropLayer, cell: IVec2) -> Vec3 {
    (cell.as_vec2() * layer.chunk + Vec2::splat(layer.chunk * 0.5)).extend(layer.z)
}

// Cells that have to be alive to cover the view.
pub(crate) fn chunk_range(position: Vec2, view: Vec2, chunk: f32) -> IRect {
    let span = Vec2::splat(chunk);

    let min = ((position - view * 0.5) / span).floor().as_ivec2();
    let max = ((position + view * 0.5) / span).floor().as_ivec2();

    IRect::from_corners(min, max)
}

// Derived from the chunk coordinate, so every client paints the same backdrop without storing
// anything per chunk, and neighbouring chunks get different treatment.
pub(crate) fn chunk_variation(layer: usize, cell: IVec2, tiles: usize) -> ChunkVariation {
    let layer = u32::try_from(layer).unwrap_or(u32::MAX);
    let tiles = u32::try_from(tiles).unwrap_or(1).max(1);

    let seed = mix(mix(spread(cell.x) ^ spread(cell.y)) ^ layer);
    let tile = usize::try_from(seed % tiles).unwrap_or(0);

    ChunkVariation {
        tile,
        flip_x: seed & (1 << 16) != 0,
        flip_y: seed & (1 << 17) != 0,
        turn: u8::try_from((seed >> 18) & 0b11).unwrap_or_default(),
    }
}

// One signed grid coordinate, mixed by bit pattern so no cell can be dropped by a shift.
const fn spread(value: i32) -> u32 {
    mix(u32::from_le_bytes(value.to_le_bytes()))
}

// Murmur-style finalizer, so the regular chunk grid does not hash into visible patterns.
const MIX_MULT_A: u32 = 2_654_435_769;
const MIX_MULT_B: u32 = 2_246_822_507;

const fn mix(value: u32) -> u32 {
    let scrambled = value.wrapping_mul(MIX_MULT_A);
    let folded = scrambled ^ (scrambled >> 16);

    let spread = folded.wrapping_mul(MIX_MULT_B);
    spread ^ (spread >> 13)
}

// One palette swatch per layer, so the streaming grid of each depth is distinguishable.
#[cfg(feature = "dev")]
const CHUNK_OUTLINE_COLORS: [Color; 4] = [
    Palette::Citron.color(),
    Palette::Mint.color(),
    Palette::Rose.color(),
    Palette::Tan.color(),
];

// Outlines of every live backdrop chunk, on the debug render layer. Runs after the layout pass
// so the outlines follow the same anchor offsets the chunks move by that frame. Gizmos are only
// available where the gizmo plugin runs, so the outlines idle in gizmo-less apps such as tests.
#[cfg(feature = "dev")]
pub(crate) fn draw_chunk_outlines(
    chunks: Query<(&BackdropChunk, &Transform)>,
    mut gizmos: Option<Gizmos<PhysicsGizmos>>,
) {
    let Some(gizmos) = gizmos.as_mut() else {
        return;
    };

    for (chunk, transform) in &chunks {
        let Some(&color) = CHUNK_OUTLINE_COLORS.get(chunk.layer) else {
            continue;
        };

        // The chunk rotation is a quarter turn in z, which maps straight onto a 2d rotation.
        let (_, _, yaw) = transform.rotation.to_euler(EulerRot::XYZ);
        gizmos.rect_2d(
            Isometry2d::new(transform.translation.xy(), Rot2::radians(yaw)),
            Vec2::splat(BACKDROP_LAYERS[chunk.layer].chunk),
            color,
        );
    }
}
