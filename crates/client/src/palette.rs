use bevy::prelude::Color;

// Every colour the client draws comes from the sixteen-swatch nebulaspace palette
// (https://lospec.com/palette-list/nebulaspace), so the code and the art under `assets/sprites`
// stay on one hue wheel instead of drifting apart. Variants keep the palette's own order and are
// named after their hue rather than their job, so a swatch can change job without being renamed.
// The complete set is listed even where no job uses a swatch yet, so this stays the palette
// rather than a running list of what happens to be drawn today. `tests::palette` pins every
// variant to the published hex.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "The enum is the whole palette, so swatches without a job yet stay listed"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Palette {
    Forest,    // #0a401a
    Moss,      // #6d852c
    Olive,     // #b3a724
    Citron,    // #e6eb6a
    Cream,     // #ede8e1
    Mint,      // #a7dbbb
    Teal,      // #5d858c
    Indigo,    // #3d476e
    Navy,      // #32244d
    PlumBlack, // #27142b
    Sand,      // #d6c2ba
    Tan,       // #bf9684
    Rose,      // #a66372
    Mulberry,  // #733754
    Plum,      // #451e3e
    Wine,      // #2e0f29
}

impl Palette {
    // The swatch as a rendered colour. `const` so swatch lists can be `const` too.
    pub(crate) const fn color(self) -> Color {
        match self {
            Self::Forest => Color::srgb_u8(10, 64, 26),
            Self::Moss => Color::srgb_u8(109, 133, 44),
            Self::Olive => Color::srgb_u8(179, 167, 36),
            Self::Citron => Color::srgb_u8(230, 235, 106),
            Self::Cream => Color::srgb_u8(237, 232, 225),
            Self::Mint => Color::srgb_u8(167, 219, 187),
            Self::Teal => Color::srgb_u8(93, 133, 140),
            Self::Indigo => Color::srgb_u8(61, 71, 110),
            Self::Navy => Color::srgb_u8(50, 36, 77),
            Self::PlumBlack => Color::srgb_u8(39, 20, 43),
            Self::Sand => Color::srgb_u8(214, 194, 186),
            Self::Tan => Color::srgb_u8(191, 150, 132),
            Self::Rose => Color::srgb_u8(166, 99, 114),
            Self::Mulberry => Color::srgb_u8(115, 55, 84),
            Self::Plum => Color::srgb_u8(69, 30, 62),
            Self::Wine => Color::srgb_u8(46, 15, 41),
        }
    }
}
