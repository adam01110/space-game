use bevy_extended_ui_macros::ui_component;

#[ui_component]
pub struct MenuComponent {
    pub template_name: &'static str,
    pub template_file: &'static str,
    pub styles: &'static [&'static str],
}

pub const MENU_COMPONENT: MenuComponent = MenuComponent {
    template_name: "app-menu",
    template_file: "menu.component.html",
    styles: &["menu.css"],
};
