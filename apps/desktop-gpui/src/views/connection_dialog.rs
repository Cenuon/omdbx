use crate::*;
use super::common::*;
use gpui_kit::component::Disableable;

impl Omdbx {
    pub(crate) fn connection_dialog_overlay(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(dialog) = self.connection_dialog.as_ref() else {
            return div().into_any_element();
        };
        let colors = cx.theme().colors;
        match dialog.step {
            ConnectionDialogStep::Pick => self.connection_type_picker(dialog, colors, cx),
            ConnectionDialogStep::Form => self.connection_form(dialog, colors, cx),
        }
    }

    fn connection_type_picker(
        &self,
        dialog: &NewConnectionDialog,
        colors: gpui_kit::component::ThemeColor,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let muted = colors.muted_foreground;
        let keyword = dialog.search.read(cx).value().trim().to_lowercase();
        let visible: Vec<DatabaseTypeSpec> = DATABASE_TYPES
            .iter()
            .copied()
            .filter(|spec| {
                if keyword.is_empty() {
                    spec.category == dialog.category
                } else {
                    spec.label.to_lowercase().contains(&keyword)
                        || spec.key.to_lowercase().contains(&keyword)
                }
            })
            .collect();

        let mut rail = v_flex().w(px(150.)).flex_none().gap(px(3.));
        for category in DATABASE_CATEGORIES {
            let active = keyword.is_empty() && dialog.category == category;
            rail = rail.child(
                div()
                    .id(format!("connection-category-{}", category.label()))
                    .h(px(33.))
                    .w_full()
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded(px(5.))
                    .cursor_pointer()
                    .text_size(px(12.5))
                    .when(active, |this| this.bg(colors.secondary))
                    .when(!active, |this| {
                        this.hover(|style| style.bg(colors.sidebar_accent))
                    })
                    .child(category.label())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.set_connection_category(category, window, cx)
                    })),
            );
        }

        let mut grid = h_flex().w_full().flex_wrap().items_start().gap(px(6.));
        if visible.is_empty() {
            grid = grid.child(
                div()
                    .py_6()
                    .w_full()
                    .text_center()
                    .text_size(px(12.))
                    .text_color(muted)
                    .child("没有匹配的数据库类型"),
            );
        }
        for spec in visible {
            let selected = dialog.selected.key == spec.key;
            grid = grid.child(
                v_flex()
                    .id(format!("connection-type-{}", spec.key))
                    .w(px(124.))
                    .h(px(76.))
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .gap(px(5.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(if selected {
                        colors.primary
                    } else {
                        colors.border
                    })
                    .bg(if selected {
                        colors.secondary
                    } else {
                        colors.background
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(colors.sidebar_accent))
                    .child(
                        connection_icon(spec.icon, true)
                            .with_size(px(24.)),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_h(px(30.))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_center()
                            .text_size(px(11.5))
                            .line_height(px(14.))
                            .child(spec.label),
                    )
                    .on_click(cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
                        this.select_connection_type(spec.key, cx);
                        if event.click_count() >= 2 {
                            this.show_connection_form(window, cx);
                        }
                    })),
            );
        }

        let search_box = h_flex()
            .h(px(32.))
            .w_full()
            .px_2()
            .gap_2()
            .items_center()
            .rounded(px(6.))
            .border_1()
            .border_color(colors.border)
            .bg(colors.background)
            .child(
                Icon::new(AssetIcon::Search)
                    .with_size(px(15.))
                    .text_color(muted),
            )
            .child(
                Input::new(&dialog.search)
                    .appearance(false)
                    .bordered(false)
                    .text_size(px(12.5))
                    .cleanable(true)
                    .w_full(),
            );

        let body = v_flex()
            .w_full()
            .min_w_0()
            .h(px(364.))
            .gap(px(6.))
            .child(search_box)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .gap_2()
                    .child(rail)
                    .child(
                        div()
                            .id("connection-type-grid")
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_y_scroll()
                            .child(grid),
                    ),
            )
            .into_any_element();

        let selected_label = dialog.selected.label;
        let cancel = cx.entity().downgrade();
        let next = cx.entity().downgrade();
        let footer = h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(muted)
                    .child(format!("已选择：{}", selected_label)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("connection-picker-cancel")
                            .ghost()
                            .with_size(gpui_kit::component::Size::Small)
                            .min_w(px(72.))
                            .label("取消")
                            .on_click(move |_, _, cx| {
                                let _ = cancel.update(cx, |this, cx| {
                                    this.close_connection_dialog(cx)
                                });
                            }),
                    )
                    .child(
                        Button::new("connection-picker-next")
                            .primary()
                            .with_size(gpui_kit::component::Size::Small)
                            .min_w(px(84.))
                            .label("下一步")
                            .on_click(move |_, window, cx| {
                                let _ = next.update(cx, |this, cx| {
                                    this.show_connection_form(window, cx)
                                });
                            }),
                    ),
            )
            .into_any_element();

        ui::dialog::modal(
            px(860.),
            "新建连接",
            "选择数据库类型，双击或点击下一步继续",
            body,
            footer,
            colors,
        )
    }

    fn connection_form(
        &self,
        dialog: &NewConnectionDialog,
        colors: gpui_kit::component::ThemeColor,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let muted = colors.muted_foreground;
        let selected = dialog.selected;
        let weak = cx.entity().downgrade();
        let input = |state: &Entity<InputState>| {
            Input::new(state)
                .appearance(true)
                .bordered(true)
                .focus_bordered(true)
                .with_size(gpui_kit::component::Size::Medium)
                .rounded(px(8.))
                .text_size(px(12.5))
                .w_full()
        };
        let password_input = input(&dialog.password).mask_toggle();
        let field = |label: &'static str,
                     hint: Option<&'static str>,
                     control: gpui_kit::AnyElement| {
            v_flex()
                .w_full()
                .min_w_0()
                .gap(px(6.))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.foreground)
                                .child(label),
                        )
                        .children(hint.map(|hint| {
                            div()
                                .text_size(px(10.5))
                                .text_color(muted)
                                .child(hint)
                        })),
                )
                .child(control)
                .into_any_element()
        };

        let type_card = h_flex()
            .id("connection-form-type")
            .w_full()
            .h(px(54.))
            .px(px(10.))
            .gap(px(10.))
            .items_center()
            .rounded(px(10.))
            .border_1()
            .border_color(colors.border)
            .bg(colors.secondary)
            .cursor_pointer()
            .hover(|style| style.bg(colors.secondary_active))
            .child(
                div()
                    .w(px(34.))
                    .h(px(34.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(9.))
                    .bg(connection_color(&selected.key).opacity(0.10))
                    .child(connection_icon(selected.icon, true).with_size(px(22.))),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.foreground)
                            .child(selected.label),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(muted)
                            .child(selected.category.label()),
                    ),
            )
            .child(div().flex_1())
            .child(
                h_flex()
                    .flex_none()
                    .gap(px(2.))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(muted)
                            .child("更换类型"),
                    )
                    .child(
                        Icon::new(AssetIcon::ChevronRight)
                            .with_size(px(12.))
                            .text_color(muted),
                    ),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.show_connection_types(window, cx)
            }));

        let host_row: gpui_kit::AnyElement = if selected.local_file {
            field(
                "数据库文件",
                None,
                input(&dialog.host).into_any_element(),
            )
        } else {
            h_flex()
                .w_full()
                .items_start()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(field("主机地址", None, input(&dialog.host).into_any_element())),
                )
                .child(div().w(px(112.)).flex_none().child(field(
                    "端口",
                    None,
                    input(&dialog.port).into_any_element(),
                )))
                .into_any_element()
        };

        let ssl = {
            let weak = weak.clone();
            Checkbox::new("connection-ssl")
                .checked(dialog.ssl)
                .label("启用 TLS/SSL")
                .on_click(move |checked, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        if let Some(dialog) = this.connection_dialog.as_mut() {
                            dialog.ssl = *checked;
                        }
                        cx.notify();
                    });
                })
        };
        let save_password = {
            let weak = weak.clone();
            Checkbox::new("connection-save-password")
                .checked(dialog.save_password)
                .label("保存密码")
                .on_click(move |checked, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        if let Some(dialog) = this.connection_dialog.as_mut() {
                            dialog.save_password = *checked;
                        }
                        cx.notify();
                    });
                })
        };

        let mut form = v_flex()
            .w_full()
            .min_w_0()
            .gap(px(12.))
            .child(type_card)
            .child(field(
                "连接名称",
                Some("留空自动命名"),
                input(&dialog.name).into_any_element(),
            ));
        form = form
            .child(host_row);
        if !selected.local_file {
            form = form
                .child(
                    h_flex()
                        .w_full()
                        .items_start()
                        .gap(px(8.))
                        .child(
                            div().flex_1().min_w_0().child(field(
                                "用户名",
                                None,
                                input(&dialog.username).into_any_element(),
                            )),
                        )
                        .child(
                            div().flex_1().min_w_0().child(field(
                                "密码",
                                None,
                                password_input.into_any_element(),
                            )),
                        ),
                )
                .child(field(
                    "默认数据库",
                    Some("可选"),
                    input(&dialog.database).into_any_element(),
                ));
        } else if selected.key == "sqlite" {
            form = form.child(field(
                "加密密钥",
                Some("可选"),
                password_input.into_any_element(),
            ));
        }
        form = form.child(
            h_flex()
                .w_full()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(40.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .bg(colors.secondary)
                        .child(ssl),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(40.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .bg(colors.secondary)
                        .child(save_password),
                ),
        );
        if let Some((ok, message)) = dialog.result.clone() {
            let tone: Hsla = if ok {
                gpui_kit::rgb(0x2F9E44).into()
            } else {
                colors.danger_foreground
            };
            let result_icon = if ok { AssetIcon::Check } else { AssetIcon::X };
            form = form.child(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap(px(8.))
                    .p(px(9.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(tone.opacity(0.20))
                    .bg(tone.opacity(0.07))
                    .child(
                        Icon::new(result_icon)
                            .with_size(px(13.))
                            .text_color(tone),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.5))
                            .line_height(px(16.))
                            .text_color(tone)
                            .child(message),
                    ),
            );
        }

        let cancel = cx.entity().downgrade();
        let test = cx.entity().downgrade();
        let save = cx.entity().downgrade();
        let busy = dialog.busy;
        let footer = h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(colors.primary.opacity(0.55)))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(muted)
                            .child("保存前建议先测试连接"),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(
                        Button::new("connection-form-cancel")
                            .ghost()
                            .with_size(gpui_kit::component::Size::Medium)
                            .min_w(px(72.))
                            .label("取消")
                            .disabled(busy.is_some())
                            .on_click(move |_, _, cx| {
                                let _ = cancel.update(cx, |this, cx| {
                                    this.close_connection_dialog(cx)
                                });
                            }),
                    )
                    .child(
                        Button::new("connection-form-test")
                            .secondary()
                            .with_size(gpui_kit::component::Size::Medium)
                            .min_w(px(104.))
                            .icon(Icon::new(AssetIcon::Play).with_size(px(13.)))
                            .label(if busy == Some(ConnectionDialogBusy::Test) {
                                "测试中…"
                            } else {
                                "测试连接"
                            })
                            .disabled(busy.is_some())
                            .on_click(move |_, _, cx| {
                                let _ = test.update(cx, |this, cx| this.test_connection_dialog(cx));
                            }),
                    )
                    .child(
                        Button::new("connection-form-save")
                            .primary()
                            .with_size(gpui_kit::component::Size::Medium)
                            .min_w(px(104.))
                            .label(if busy == Some(ConnectionDialogBusy::Save) {
                                "保存中…"
                            } else {
                                "保存连接"
                            })
                            .disabled(busy.is_some())
                            .on_click(move |_, _, cx| {
                                let _ = save.update(cx, |this, cx| this.save_connection_dialog(cx));
                            }),
                    ),
            )
            .into_any_element();

        let subtitle = h_flex()
            .gap(px(4.))
            .items_center()
            .child(connection_icon(selected.icon, true).with_size(px(12.)))
            .child(format!("{} · {}", selected.label, selected.category.label()));
        let form = div()
            .id("connection-form-scroll")
            .w_full()
            .min_w_0()
            .max_h(px(400.))
            .overflow_y_scroll()
            .child(form);
        ui::dialog::modal(
            px(680.),
            "连接配置",
            subtitle,
            form.into_any_element(),
            footer,
            colors,
        )
    }
}
