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
pub(crate) const FRAME_SVG: &str = include_str!("../../../assets/ui/frame.svg");
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

// Keep the authored SVG in assets and replace only its paths and viewBox at resize, so the
// source stays a valid, viewable 800x600 SVG.
fn replace_path(svg: &str, id: &str, path: &str) -> Option<String> {
    let marker = format!("id=\"{id}\" d=\"");
    let (prefix, remainder) = svg.split_once(&marker)?;
    let (_, suffix) = remainder.split_once('"')?;

    Some(format!("{prefix}{marker}{path}\"{suffix}"))
}

pub(crate) fn frame_svg(width: f32, height: f32) -> Option<String> {
    let outline = frame_outline(width, height);
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

fn frame_outline(width: f32, height: f32) -> String {
    let inset = INSET.min(width.min(height) / 2.0);
    let corner = CORNER.min((width.min(height) / 2.0 - inset).max(0.0));
    let near = inset + corner;
    let far_x = width - near;
    let far_y = height - near;
    let right = width - inset;
    let bottom = height - inset;

    format!(
        "M{near} {inset} H{far_x} L{right} {near} V{far_y} L{far_x} {bottom} H{near} L{inset} {far_y} V{near} Z"
    )
}

pub(crate) fn update_frame(
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
    if size.0 != 0 && size.1 != 0 {
        let key = (size.0, size.1, window.scale_factor().to_bits());
        refresh_frame_image(
            &window,
            size,
            key,
            &mut images,
            &mut image_handle,
            &mut last_size,
        );

        if let Some(handle) = image_handle.as_ref() {
            set_frame_node(&mut image_node, handle);
        }
    }
}

fn refresh_frame_image(
    window: &Window,
    size: (u32, u32),
    key: (u32, u32, u32),
    images: &mut Assets<Image>,
    handle: &mut Option<Handle<Image>>,
    last_size: &mut (u32, u32, u32),
) {
    if *last_size == key && handle.is_some() {
        return;
    }
    if let Some(image) = render_frame(window, size) {
        store_frame_image(images, handle, image);
        *last_size = key;
    }
}

fn set_frame_node(node: &mut ImageNode, handle: &Handle<Image>) {
    if node.image != *handle {
        node.image = handle.clone();
    }
    node.image_mode = NodeImageMode::Stretch;
}

fn render_frame(window: &Window, size: (u32, u32)) -> Option<Image> {
    let Some(svg) = frame_svg(window.width(), window.height()) else {
        warn!("Frame SVG template is missing its named paths");
        return None;
    };
    let Ok(tree) = Tree::from_data(svg.as_bytes(), &Options::default()) else {
        warn!("Could not parse the frame SVG");
        return None;
    };
    let Some(mut pixmap) = Pixmap::new(size.0, size.1) else {
        warn!("Could not allocate the frame image");
        return None;
    };
    let scale = Transform::from_scale(window.scale_factor(), window.scale_factor());
    resvg::render(&tree, scale, &mut pixmap.as_mut());
    Some(Image::new(
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
    ))
}

fn store_frame_image(images: &mut Assets<Image>, handle: &mut Option<Handle<Image>>, image: Image) {
    if let Some(existing) = handle.as_ref()
        && let Some(mut old) = images.get_mut(existing)
    {
        *old = image;
        return;
    }
    *handle = Some(images.add(image));
}
