use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};
use bevy_extended_ui::{ExtendedUiPlugin, styles::CssID};
use bevy_resvg::resvg::{
    self,
    tiny_skia::Pixmap,
    usvg::{Options, Transform, Tree},
};

const INSET: f32 = 30.0;
const CORNER: f32 = 200.0;
const FRAME_SVG: &str = include_str!("../../../assets/ui/frame.svg");
const BASE_SIZE: &str = "width=\"800\" height=\"600\" viewBox=\"0 0 800 600\"";

pub(super) struct NativeFramePlugin;

impl Plugin for NativeFramePlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<ExtendedUiPlugin>());
        let component = &crate::frame_component::FRAME_COMPONENT;
        debug_assert_eq!(component.template_name, "app-frame");
        debug_assert_eq!(component.template_file, "frame.component.html");
        debug_assert_eq!(component.styles, &["frame.css"]);
        app.add_systems(Update, update_frame);
    }
}

// Keep the authored SVG in assets and replace only its paths and viewBox at resize.
// The source remains a valid, viewable 800x600 SVG on its own.
fn replace_path(svg: &str, id: &str, path: &str) -> Option<String> {
    let marker = format!("id=\"{id}\" d=\"");
    let (prefix, remainder) = svg.split_once(&marker)?;
    let (_, suffix) = remainder.split_once('"')?;
    Some(format!("{prefix}{marker}{path}\"{suffix}"))
}

fn frame_svg(width: f32, height: f32) -> Option<String> {
    let inset = INSET.min(width / 2.0).min(height / 2.0);
    let corner = CORNER
        .min((width / 2.0 - inset).max(0.0))
        .min((height / 2.0 - inset).max(0.0));
    let near = inset + corner;
    let far_x = width - near;
    let far_y = height - near;
    let right = width - inset;
    let bottom = height - inset;
    let outline = format!(
        "M{near} {inset} H{far_x} L{right} {near} V{far_y} L{far_x} {bottom} H{near} L{inset} {far_y} V{near} Z"
    );
    let (prefix, suffix) = FRAME_SVG.split_once(BASE_SIZE)?;
    let sized = format!(
        "{prefix}width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"{suffix}"
    );
    let masked = replace_path(
        &sized,
        "frame-mask",
        &format!("M0 0 H{width} V{height} H0 Z {outline}"),
    )?;
    replace_path(&masked, "frame-outline", &outline)
}

fn update_frame(
    window: Single<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&CssID, &mut ImageNode)>,
    mut images: ResMut<Assets<Image>>,
    mut image_handle: Local<Option<Handle<Image>>>,
    mut last_size: Local<(u32, u32, u32)>,
) {
    let Some((_, mut image_node)) = nodes.iter_mut().find(|(id, _)| id.0 == "frame-art") else {
        return;
    };
    let size = (window.physical_width(), window.physical_height());
    if size.0 == 0 || size.1 == 0 {
        return;
    }
    let key = (size.0, size.1, window.scale_factor().to_bits());

    if *last_size != key || image_handle.is_none() {
        let Some(svg) = frame_svg(window.width(), window.height()) else {
            warn!("Frame SVG template is missing its named paths");
            return;
        };
        let Ok(tree) = Tree::from_data(svg.as_bytes(), &Options::default()) else {
            warn!("Could not parse the frame SVG");
            return;
        };
        let Some(mut pixmap) = Pixmap::new(size.0, size.1) else {
            warn!("Could not allocate the frame image");
            return;
        };
        let scale = Transform::from_scale(window.scale_factor(), window.scale_factor());
        resvg::render(&tree, scale, &mut pixmap.as_mut());
        let image = Image::new(
            Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            // SVG and CSS colors are sRGB; treating these bytes as linear brightens the mask.
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        if let Some(handle) = image_handle.as_ref() {
            if let Some(mut old) = images.get_mut(handle) {
                *old = image;
            } else {
                *image_handle = Some(images.add(image));
            }
        } else {
            *image_handle = Some(images.add(image));
        }
        *last_size = key;
    }

    if let Some(handle) = image_handle.as_ref() {
        if image_node.image != *handle {
            image_node.image = handle.clone();
        }
        image_node.image_mode = NodeImageMode::Stretch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

            assert_eq!(alpha(0, 0), 255, "masked corner");
            assert!(alpha(30, 230) > 0, "left cut joins the vertical edge");
            assert!(alpha(230, 30) > 0, "top cut joins the horizontal edge");
            assert_eq!(alpha(width / 2, height / 2), 0, "clear play area");
        }
    }
}
