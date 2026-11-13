use gpui_kit::component::{h_flex, ThemeColor};
use gpui_kit::prelude::*;
use gpui_kit::{px, App, ClickEvent, Stateful, Window};

use super::tokens::ICON_BUTTON;

/// One square icon button: titlebar and toolbar buttons share this so their
/// hit area, radius and hover fill cannot drift apart.
pub fn action_button(
    id: &'static str,
    child: gpui_kit::AnyElement,
    enabled: bool,
    colors: ThemeColor,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<gpui_kit::Div> {
    let button = h_flex()
        .id(id)
        .h(px(ICON_BUTTON))
        .w(px(ICON_BUTTON))
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_md()
        .child(child);
    if enabled {
        button
            .cursor_pointer()
            .hover(|style| style.bg(colors.secondary_active))
            .on_click(on_click)
    } else {
        button.opacity(0.35).cursor_default()
    }
}

pub fn icon_button(
    id: &'static str,
    child: gpui_kit::AnyElement,
    colors: ThemeColor,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<gpui_kit::Div> {
    h_flex()
        .id(id)
        .h(px(ICON_BUTTON))
        .w(px(ICON_BUTTON))
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(colors.secondary_active))
        .child(child)
        .on_click(on_click)
}
