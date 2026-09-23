use bevy_extended_ui_macros::ui_component;

#[ui_component]
pub struct FrameComponent {
    pub template_name: &'static str,
    pub template_file: &'static str,
    pub styles: &'static [&'static str],
}

pub const FRAME_COMPONENT: FrameComponent = FrameComponent {
    template_name: "app-frame",
    template_file: "frame.component.html",
    styles: &["frame.css"],
};
