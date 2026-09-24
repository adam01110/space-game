use bevy::{
    asset::AssetPlugin,
    camera::{CameraPlugin, CameraUpdateSystems, RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::{camera::camera_system, texture::ManualTextureViews},
    transform::TransformPlugin,
    window::PrimaryWindow,
};
use bevy_extended_ui::{
    ExtendedCam, ExtendedUiConfiguration, ExtendedUiPlugin,
    framework::ExtendedFrameworkConfiguration,
    html::{HtmlSource, HtmlStructureMap},
    io::HtmlAsset,
    styles::CssID,
};
use lightyear::prelude::input::native::InputMarker;

use space_game_protocol::{Player, PlayerHealth, PlayerInput};

use crate::{
    camera::{CANVAS_CAMERA_SCALE, CanvasCamera, GameplayCamera, GameplayCanvas, PIXEL_SIZE},
    plugins::browser_hud_html,
    remote_health::{RemoteHealthBar, RemoteHealthPlugin},
};

#[test]
fn browser_template_builds_the_actual_extended_ui_nodes() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "../../assets".into(),
            ..default()
        },
        bevy::input::InputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::text::TextPlugin,
        bevy::image::ImagePlugin::default(),
    ))
    .init_asset::<bevy::shader::Shader>()
    .init_asset::<Image>()
    .init_resource::<bevy::picking::hover::HoverMap>()
    .init_asset::<TextureAtlasLayout>()
    .add_message::<bevy::window::WindowResized>();
    app.insert_resource(ExtendedFrameworkConfiguration {
        assets_component_root: "ui".into(),
        rust_component_root: format!("{}/src", env!("CARGO_MANIFEST_DIR")),
        asset_root_fs_path: format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")),
        index_html_file: "index.html".into(),
    });
    app.insert_resource(ExtendedUiConfiguration {
        camera: ExtendedCam::None,
        framework_components_path: "ui".into(),
        ..default()
    });
    app.add_plugins(ExtendedUiPlugin);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let browser_html = browser_hud_html();
    assert!(!browser_html.contains("app-menu"));
    let html = app
        .world_mut()
        .resource_mut::<Assets<HtmlAsset>>()
        .add(HtmlAsset {
            html: browser_html,
            stylesheets: Vec::new(),
        });
    app.world_mut().spawn(HtmlSource::from_handle(html));
    for _ in 0..20 {
        app.update();
    }
    let mut ids = app.world_mut().query::<(&CssID, &Node)>();
    let nodes: Vec<_> = ids.iter(app.world()).map(|(id, _)| id.0.as_str()).collect();
    assert!(
        app.world()
            .resource::<HtmlStructureMap>()
            .html_map
            .contains_key("browser-hud")
    );
    assert!(nodes.contains(&"hud-remote-health"), "{nodes:?}");
    assert!(nodes.contains(&"hud-health-fill"), "{nodes:?}");
    let (_, layer) = ids
        .iter(app.world())
        .find(|(id, _)| id.0 == "hud-remote-health")
        .expect("framework health layer");
    assert_eq!(layer.width, Val::Percent(100.0));
    assert_eq!(layer.height, Val::Percent(100.0));
    assert!(nodes.contains(&"hud-map-ring"), "{nodes:?}");
    let (_, track) = ids
        .iter(app.world())
        .find(|(id, _)| id.0 == "hud-health-track")
        .expect("framework health track");
    assert_eq!(track.width, Val::Px(140.0));
}

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TransformPlugin,
        CameraPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Mesh>()
    .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
    .init_resource::<ManualTextureViews>()
    .add_message::<bevy::window::WindowResized>()
    .add_message::<bevy::window::WindowCreated>()
    .add_message::<bevy::window::WindowScaleFactorChanged>()
    .add_systems(PostUpdate, camera_system.in_set(CameraUpdateSystems))
    .add_plugins(RemoteHealthPlugin);
    app.world_mut().spawn((
        Window {
            resolution: (800, 600).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 200,
                height: 150,
                ..default()
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[0, 0, 0, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        ));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: PIXEL_SIZE,
            ..OrthographicProjection::default_2d()
        }),
        RenderTarget::Image(image.clone().into()),
        GameplayCamera,
        RenderLayers::layer(0),
    ));
    app.world_mut().spawn((
        Sprite::from_image(image),
        Transform::default(),
        GameplayCanvas,
    ));
    app.world_mut().spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: CANVAS_CAMERA_SCALE,
            ..OrthographicProjection::default_2d()
        }),
        CanvasCamera,
    ));
    let root = app
        .world_mut()
        .spawn((CssID("hud-remote-health".into()), Node::default()))
        .id();
    app.world_mut()
        .spawn((Player, InputMarker::<PlayerInput>::default()));
    (app, root)
}

fn assert_px(actual: Val, expected: f32) {
    let Val::Px(actual) = actual else {
        panic!("expected pixel dimension");
    };
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

#[test]
fn bars_use_render_cameras_and_follow_remote_health() {
    let (mut app, root) = setup();
    let remote = app
        .world_mut()
        .spawn((
            Player,
            PlayerHealth(50),
            Transform::from_xyz(100.0, 40.0, 0.0),
        ))
        .id();
    app.update();
    let mut tracks = app
        .world_mut()
        .query_filtered::<(Entity, &Node), With<RemoteHealthBar>>();
    let (track, node) = tracks.single(app.world()).unwrap();
    assert_eq!(node.left, Val::Px(400.0 + 100.0 - 17.55));
    assert_eq!(node.top, Val::Px(300.0 - 40.0 + 23.4 + 4.0));
    assert_px(node.width, 35.1);
    assert_eq!(app.world().get::<Children>(root).unwrap().len(), 1);
    let fill = app.world().get::<Children>(track).unwrap()[0];
    assert_px(app.world().get::<Node>(fill).unwrap().width, 17.55);

    app.world_mut().get_mut::<PlayerHealth>(remote).unwrap().0 = 100;
    app.world_mut()
        .get_mut::<Transform>(remote)
        .unwrap()
        .translation
        .x = 200.0;
    app.update();
    assert_eq!(
        app.world().get::<Node>(track).unwrap().left,
        Val::Px(400.0 + 200.0 - 17.55)
    );
    assert_px(app.world().get::<Node>(fill).unwrap().width, 35.1);

    let mut canvas = app
        .world_mut()
        .query_filtered::<(&mut Camera, &mut Projection), With<CanvasCamera>>();
    let (mut camera, mut projection) = canvas.single_mut(app.world_mut()).unwrap();
    camera.viewport = Some(bevy::camera::Viewport {
        physical_position: UVec2::new(100, 50),
        physical_size: UVec2::new(400, 300),
        ..default()
    });
    let Projection::Orthographic(ref mut orthographic) = *projection else {
        panic!("expected orthographic camera");
    };
    orthographic.scale = CANVAS_CAMERA_SCALE * 2.0;
    app.update();
    let track_node = app.world().get::<Node>(track).unwrap();
    assert_px(track_node.left, 400.0 - 35.1 / 4.0);
    assert_px(track_node.width, 35.1 / 2.0);
    assert_px(app.world().get::<Node>(fill).unwrap().width, 35.1 / 2.0);

    app.world_mut().despawn(remote);
    app.update();
    assert!(tracks.iter(app.world()).next().is_none());
}
