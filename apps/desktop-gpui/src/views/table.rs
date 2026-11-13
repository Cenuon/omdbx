use crate::*;

impl Omdbx {
    pub(crate) fn table_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let page = self.table_page;
        let icon_size = px(15.);
        let dirty = self.has_pending_changes();
        let running = self.loading && self.running_query.is_some();
        let save_color = if dirty {
            gpui_kit::rgb(0x2F9E44).into()
        } else {
            muted
        };
        let cancel_color = if dirty {
            gpui_kit::rgb(0xE03131).into()
        } else {
            muted
        };

        h_flex()
            .h(px(32.))
            .flex_none()
            .px_3()
            .gap_1()
            .items_center()
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.background)
            .child(ui::button::icon_button(
                "row-add",
                Icon::new(AssetIcon::Plus)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, _, cx| this.add_row(cx)),
            ))
            .child(ui::button::icon_button(
                "row-remove",
                Icon::new(AssetIcon::Minus)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, _, cx| this.delete_selected_row(cx)),
            ))
            .child(ui::button::action_button(
                "row-save",
                Icon::new(AssetIcon::Check)
                    .with_size(icon_size)
                    .text_color(save_color)
                    .into_any_element(),
                dirty,
                colors,
                cx.listener(|this, _, window, cx| this.save_pending_edits(window, cx)),
            ))
            .child(ui::button::action_button(
                "row-cancel",
                div()
                    .text_size(px(14.))
                    .text_color(cancel_color)
                    .child("✕")
                    .into_any_element(),
                dirty,
                colors,
                cx.listener(|this, _, _, cx| this.cancel_pending_edits(cx)),
            ))
            .child(ui::button::icon_button(
                "table-reload",
                Icon::new(AssetIcon::RotateCw)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| this.load_active(window, cx)),
            ))
            .child(ui::button::action_button(
                "stop-query",
                div()
                    .w(px(11.))
                    .h(px(11.))
                    .rounded(px(2.))
                    .bg(if running { colors.foreground } else { muted })
                    .into_any_element(),
                running,
                colors,
                cx.listener(|this, _, _, cx| this.stop_query(cx)),
            ))
            .child(div().flex_1())
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(muted)
                    .child(div().child(format!("第 {page} 页")))
                    .child(ui::button::icon_button(
                        "page-prev",
                        Icon::new(AssetIcon::ChevronLeft)
                            .with_size(px(14.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| {
                            if this.table_page > 1 {
                                this.table_page -= 1;
                                this.load_active(window, cx);
                            }
                        }),
                    ))
                    .child(ui::button::icon_button(
                        "page-next",
                        Icon::new(AssetIcon::ChevronRight)
                            .with_size(px(14.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| {
                            if this.table_has_more {
                                this.table_page += 1;
                                this.load_active(window, cx);
                            }
                        }),
                    ))
                    .child(div().ml_1().child(format!("{} 行/页", ROW_LIMIT))),
            )
    }

    /// Table with an outer frame, a small gap, and a rounded content area.
    pub(crate) fn table_frame(
        &self,
        grid: &Entity<TableState<GridDelegate>>,
        colors: gpui_kit::component::ThemeColor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let header_height = self.header_height();
        let viewport = self.grid_viewport.clone();
        let app_id = cx.entity().entity_id();
        let grid_for_viewport = grid.clone();
        div()
            .size_full()
            .rounded(px(10.))
            .child(
                div()
                    .relative()
                    .size_full()
                    .rounded(px(10.))
                    .bg(colors.background)
                    .overflow_hidden()
                    // the table head is transparent, so this rounded band is
                    // what gives the content area its top corners
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h(header_height)
                            .border_t_1()
                            .border_color(colors.border)
                            .bg(gpui_kit::rgb(0xF0F0F1)),
                    )
                    .child(
                        div()
                            .size_full()
                            .on_prepaint(move |bounds, window, _| {
                                let width = f32::from(bounds.size.width);
                                if (viewport.get() - width).abs() > 0.5 {
                                    viewport.set(width);
                                    let grid = grid_for_viewport.clone();
                                    window.on_next_frame(move |_, cx| {
                                        grid.update(cx, |state, cx| state.refresh(cx));
                                        cx.notify(app_id);
                                    });
                                }
                            })
                            .child(
                                DataTable::new(grid)
                                    .bordered(false)
                                    .stripe(false)
                                    .with_size(gpui_kit::component::Size::Size(px(
                                        self.grid_row_height * self.grid_zoom,
                                    )))
                                    .header_row_height(header_height),
                            ),
                    ),
            )
    }

    pub(crate) fn empty_state(
        title: &'static str,
        hint: impl IntoElement,
        colors: gpui_kit::component::ThemeColor,
    ) -> impl IntoElement {
        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                Icon::new(AssetIcon::Table2)
                    .with_size(px(28.))
                    .text_color(colors.muted_foreground),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(colors.muted_foreground)
                    .child(hint),
            )
    }

    pub(crate) fn header_height(&self) -> gpui_kit::Pixels {
        let has_comment = self.show_comments
            && self
                .column_meta
                .iter()
                .any(|meta| !meta.comment.is_empty());
        let base = match (self.grid_show_types, has_comment) {
            (true, true) => 64.,
            (true, false) | (false, true) => 48.,
            (false, false) => ui::tokens::HEADER_ROW_BASE,
        };
        px(base * self.grid_zoom)
    }

}
