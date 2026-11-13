//! Generic modal shell. Screens provide the body and footer; the scrim,
//! panel radius, padding and title styling stay identical everywhere.

use gpui_kit::component::{h_flex, v_flex, ThemeColor};
use gpui_kit::{div, px, AnyElement, FontWeight, IntoElement, ParentElement, Styled};

use super::tokens::{DIALOG_PADDING, FONT_DIALOG_SUBTITLE, FONT_DIALOG_TITLE};

pub fn modal(
    width: gpui_kit::Pixels,
    title: impl IntoElement,
    subtitle: impl IntoElement,
    body: AnyElement,
    footer: AnyElement,
    colors: ThemeColor,
) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .p(px(12.))
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui_kit::rgba(0x0B122073))
        .child(
            v_flex()
                .w_full()
                .max_w(width)
                .min_w_0()
                .bg(colors.popover)
                .border_1()
                .border_color(colors.border)
                .rounded(px(14.))
                .shadow_lg()
                .overflow_hidden()
                .child(
                    h_flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(DIALOG_PADDING))
                        .pt(px(18.))
                        .pb(px(14.))
                        .child(
                            div()
                                .w(px(3.))
                                .h(px(20.))
                                .rounded(px(2.))
                                .bg(colors.primary),
                        )
                        .child(
                            v_flex()
                                .gap(px(3.))
                                .child(
                                    div()
                                        .text_size(px(FONT_DIALOG_TITLE))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(colors.foreground)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(FONT_DIALOG_SUBTITLE))
                                        .text_color(colors.muted_foreground)
                                        .child(subtitle),
                                ),
                        ),
                )
                .child(div().h(px(1.)).w_full().bg(colors.border))
                .child(
                    div()
                        .px(px(DIALOG_PADDING))
                        .py(px(16.))
                        .child(body),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .px(px(DIALOG_PADDING))
                        .py(px(12.))
                        .border_t_1()
                        .border_color(colors.border)
                        .child(footer),
                ),
        )
        .into_any_element()
}
