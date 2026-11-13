use crate::*;

impl Omdbx {
    pub(crate) fn column_visibility_overlay(&mut self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let colors = cx.theme().colors;
        let hidden = self
            .grid
            .as_ref()
            .map(|grid| grid.read(cx).delegate().hidden.clone())
            .unwrap_or_default();
        let columns = self.columns.clone();
        let app = cx.entity().downgrade();
        let mut list = v_flex()
            .id("column-vis-list")
            .gap(px(2.))
            .max_h(px(360.))
            .overflow_y_scroll();
        for (data, name) in columns.iter().enumerate() {
            let visible = !hidden.contains(&data);
            let weak = app.clone();
            list = list.child(
                h_flex()
                    .px(px(6.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .child(
                        Checkbox::new(("column-vis", data))
                            .checked(visible)
                            .label(name.clone())
                            .on_click(move |checked, window, cx| {
                                let _ = weak.update(cx, |this, cx| {
                                    this.toggle_column_hidden(data, !*checked, window, cx)
                                });
                            }),
                    ),
            );
        }
        let show_all = app.clone();
        let done = app.clone();
        let footer = h_flex()
            .justify_end()
            .gap(px(8.))
            .child(
                Button::new("column-vis-all")
                    .ghost()
                    .with_size(gpui_kit::component::Size::Medium)
                    .min_w(px(84.))
                    .label("全部显示")
                    .on_click(move |_, window, cx| {
                        let _ = show_all.update(cx, |this, cx| {
                            if let Some(grid) = this.grid.clone() {
                                let mut delegate = grid.read(cx).delegate().clone();
                                delegate.hidden.clear();
                                delegate.frozen = 0;
                                this.install_grid(delegate, window, cx);
                            }
                            cx.notify();
                        });
                    }),
            )
            .child(
                Button::new("column-vis-done")
                    .primary()
                    .with_size(gpui_kit::component::Size::Medium)
                    .min_w(px(84.))
                    .label("完成")
                    .on_click(move |_, _, cx| {
                        let _ = done.update(cx, |this, cx| {
                            this.column_vis_open = false;
                            cx.notify();
                        });
                    }),
            )
            .into_any_element();
        ui::dialog::modal(
            px(380.),
            "显示或隐藏列",
            "勾选需要在表格中显示的列",
            list.into_any_element(),
            footer,
            colors,
        )
    }

    pub(crate) fn width_dialog_overlay(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let colors = cx.theme().colors;
        let Some(dialog) = self.width_dialog.as_ref() else {
            return div().into_any_element();
        };
        let title = match dialog.target {
            WidthTarget::Column(_) => "设置列宽",
            WidthTarget::All => "设置所有列宽度",
            WidthTarget::RowHeight => "设置行高",
        };
        let hint = match dialog.target {
            WidthTarget::RowHeight => "按 100% 缩放基准输入行高，范围 16–120",
            _ => "按 100% 缩放基准输入列宽，范围 40–1200",
        };
        let input = dialog.input.clone();
        let error = dialog.error.clone();
        let ok = cx.entity().downgrade();
        let cancel = ok.clone();
        let body = v_flex()
            .gap(px(6.))
            .child(
                Input::new(&input)
                    .appearance(true)
                    .bordered(true)
                    .focus_bordered(true)
                    .w_full()
                    .with_size(gpui_kit::component::Size::Medium),
            )
            .when_some(error, |this, error| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(gpui_kit::rgb(0xD9534F))
                        .child(error),
                )
            })
            .into_any_element();
        let footer = h_flex()
            .justify_end()
            .gap(px(8.))
            .child(
                Button::new("width-cancel")
                    .ghost()
                    .with_size(gpui_kit::component::Size::Medium)
                    .min_w(px(84.))
                    .label("取消")
                    .on_click(move |_, _, cx| {
                        let _ = cancel.update(cx, |this, cx| {
                            this.width_dialog = None;
                            cx.notify();
                        });
                    }),
            )
            .child(
                Button::new("width-ok")
                    .primary()
                    .with_size(gpui_kit::component::Size::Medium)
                    .min_w(px(84.))
                    .label("确定")
                    .on_click(move |_, _, cx| {
                        let _ = ok.update(cx, |this, cx| this.apply_width_dialog(cx));
                    }),
            )
            .into_any_element();
        ui::dialog::modal(px(380.), title, hint, body, footer, colors)
    }

}
