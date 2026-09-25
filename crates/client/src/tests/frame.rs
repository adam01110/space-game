use bevy::{prelude::*, render::render_resource::TextureFormat, window::PrimaryWindow};
use bevy_extended_ui::styles::CssID;
use bevy_resvg::resvg::{
    self,
    tiny_skia::Pixmap,
    usvg::{Options, Transform, Tree},
};

use crate::frame::{CORNER, FRAME_SVG, INSET, frame_svg, update_frame};

#[test]
fn the_frameworks_existing_image_node_receives_the_rendered_frame() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .add_systems(Update, update_frame);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let frame = app
        .world_mut()
        .spawn((CssID("frame-art".to_owned()), ImageNode::default()))
        .id();

    app.update();

    let image_node = app.world().get::<ImageNode>(frame).expect("frame node");
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&image_node.image)
        .expect("the existing transparent image is replaced with the SVG raster");
    assert_eq!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb
    );
    assert_eq!(image_node.image_mode, NodeImageMode::Stretch);
}

#[test]
fn the_asset_is_a_valid_standalone_svg() {
    Tree::from_data(FRAME_SVG.as_bytes(), &Options::default()).expect("valid frame asset");
}

#[test]
fn the_svg_has_continuous_corners_and_a_transparent_play_area() {
    for (width, height, logical_width, logical_height) in
        [(800, 600, 800.0, 600.0), (1200, 700, 1200.0, 700.0)]
    {
        let svg = frame_svg(logical_width, logical_height).expect("frame paths");
        let tree = Tree::from_data(svg.as_bytes(), &Options::default()).expect("valid SVG");
        let mut pixels = Pixmap::new(width, height).expect("image size");
        resvg::render(&tree, Transform::default(), &mut pixels.as_mut());
        let alpha = |x: u32, y: u32| {
            let offset = ((y * width + x) * 4 + 3) as usize;
            pixels.data()[offset]
        };
        // These windows are large enough that the clamp leaves the cut at its full length; the
        // probes below fail loudly if that ever stops holding.
        let edge = u32::from(INSET);
        let near = u32::from(INSET + CORNER);

        assert_eq!(alpha(0, 0), 255, "masked corner");
        assert!(alpha(edge, near) > 0, "left cut joins the vertical edge");
        assert!(alpha(near, edge) > 0, "top cut joins the horizontal edge");
        assert_eq!(alpha(width / 2, height / 2), 0, "clear play area");
    }
}
