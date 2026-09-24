// One depth of the backdrop, a grid of square chunks that streams with the camera at the layer's
// parallax rate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BackdropLayer {
    pub(crate) sprites: &'static [&'static str],
    pub(crate) chunk: f32,
    pub(crate) z: f32,
    pub(crate) parallax: f32,
}

// Painted furthest first, so the nearest layer is drawn last. Crate-visible because the client
// tests pin it without a render app.
pub(crate) const BACKDROP_LAYERS: [BackdropLayer; 4] = [
    BackdropLayer {
        sprites: &[
            "sprites/nebula-a.png",
            "sprites/nebula-b.png",
            "sprites/nebula-c.png",
        ],
        chunk: 2048.0,
        z: -150.0,
        parallax: 0.05,
    },
    BackdropLayer {
        sprites: &["sprites/starfield-far.png"],
        chunk: 512.0,
        z: -110.0,
        parallax: 0.25,
    },
    BackdropLayer {
        sprites: &["sprites/starfield-mid.png"],
        chunk: 768.0,
        z: -80.0,
        parallax: 0.45,
    },
    BackdropLayer {
        sprites: &["sprites/starfield-near.png"],
        chunk: 1024.0,
        z: -50.0,
        parallax: 0.65,
    },
];
