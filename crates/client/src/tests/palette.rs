use bevy::prelude::Color;

use crate::palette::Palette;

// The sixteen swatches exactly as lospec publishes them, so a mistyped channel in
// `Palette::color` fails the test below instead of quietly shifting a hue.
const PUBLISHED: [(&str, Palette); 16] = [
    ("0a401a", Palette::Forest),
    ("6d852c", Palette::Moss),
    ("b3a724", Palette::Olive),
    ("e6eb6a", Palette::Citron),
    ("ede8e1", Palette::Cream),
    ("a7dbbb", Palette::Mint),
    ("5d858c", Palette::Teal),
    ("3d476e", Palette::Indigo),
    ("32244d", Palette::Navy),
    ("27142b", Palette::PlumBlack),
    ("d6c2ba", Palette::Sand),
    ("bf9684", Palette::Tan),
    ("a66372", Palette::Rose),
    ("733754", Palette::Mulberry),
    ("451e3e", Palette::Plum),
    ("2e0f29", Palette::Wine),
];

#[test]
fn every_swatch_matches_the_published_palette() {
    for (hex, swatch) in PUBLISHED {
        assert_eq!(
            color_from_hex(hex),
            Some(swatch.color()),
            "#{hex} is not {swatch:?}"
        );
    }
}

// `None` for anything that is not exactly three byte pairs, so a malformed table entry fails the
// comparison above instead of panicking here.
fn color_from_hex(hex: &str) -> Option<Color> {
    let (pairs, _) = hex.as_bytes().as_chunks::<2>();
    let [first, second, third] = pairs else {
        return None;
    };

    let [red, green, blue] = [channel(*first)?, channel(*second)?, channel(*third)?];
    Some(Color::srgb_u8(red, green, blue))
}

// One byte pair of a swatch as a channel.
fn channel(pair: [u8; 2]) -> Option<u8> {
    let [high, low] = pair;
    Some(nibble(high)? * 16 + nibble(low)?)
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
