use bevy_extended_ui_macros::ui_component;

#[ui_component]
pub struct HudComponent {
    pub template_name: &'static str,
    pub template_file: &'static str,
    pub styles: &'static [&'static str],
}

pub const HUD_COMPONENT: HudComponent = HudComponent {
    template_name: "app-hud",
    template_file: "hud.component.html",
    styles: &["hud.css"],
};
