// One depth of the backdrop, painted as a grid of square chunks that streams with the camera at
// the layer's own parallax rate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BackdropLayer {
    pub(crate) sprites: &'static [&'static str],
    pub(crate) chunk: f32,
    pub(crate) z: f32,
    pub(crate) parallax: f32,
}

// Painted furthest first, so the nearest layer is drawn last. The geometry is crate-visible
// because the client tests pin it without a render app.
pub(crate) const BACKDROP_LAYERS: [BackdropLayer; 4] = [
    BackdropLayer {
        sprites: &[
            "sprites/nebula-a.svg",
            "sprites/nebula-b.svg",
            "sprites/nebula-c.svg",
        ],
        chunk: 2048.0,
        z: -150.0,
        parallax: 0.05,
    },
    BackdropLayer {
        sprites: &["sprites/starfield-far.svg"],
        chunk: 512.0,
        z: -110.0,
        parallax: 0.25,
    },
    BackdropLayer {
        sprites: &["sprites/starfield-mid.svg"],
        chunk: 768.0,
        z: -80.0,
        parallax: 0.45,
    },
    BackdropLayer {
        sprites: &["sprites/starfield-near.svg"],
        chunk: 1024.0,
        z: -50.0,
        parallax: 0.65,
    },
];
