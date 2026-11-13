use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{px, Context};

use super::tokens::FONT_MENU;

/// The single source of truth for every popup menu: same font, same size as
/// the tab context menu.
pub fn app_menu(menu: PopupMenu, cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.font_family(cx.theme().font_family.clone())
        .text_size(px(FONT_MENU))
}
