use crate::*;

impl Omdbx {
    pub(crate) fn tab_bar(&self, leading_toggle: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        h_flex()
            .h(px(44.))
            .flex_none()
            .w_full()
            .min_w_0()
            .items_center()
            .gap_1()
            .when(leading_toggle, |this| {
                this.pl(px(80.))
                    .child(self.sidebar_toggle(cx))
                    .child(div().w(px(4.)))
            })
            .when(!leading_toggle, |this| this.pl(px(12.)))
            .pr(px(12.))
            .child(
                h_flex()
                    .id("tab-scroll")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_center()
                    .gap_1()
                    .overflow_x_scroll()
                    .track_scroll(&self.tab_scroll)
                    .children(self.tabs.iter().enumerate().map(|(index, item)| {
                        let is_active = Some(index) == self.active && !self.show_query;
                        let title = item.title.clone();
                        let close_index = index;
                        self.tab_chip(
                            format!("tab-{}", item.key),
                            AssetIcon::Table2,
                            title,
                            is_active,
                            Some(CloseTarget::Table(close_index)),
                            colors,
                            cx,
                        )
                    }))
                    .children(self.queries.iter().enumerate().map(|(index, query)| {
                        let is_active = self.show_query && self.active_query == Some(index);
                        self.tab_chip(
                            format!("tab-{}", query.key),
                            AssetIcon::SquareTerminal,
                            query.title.clone(),
                            is_active,
                            Some(CloseTarget::Query(index)),
                            colors,
                            cx,
                        )
                    }))
                    // the empty strip after the tabs drags the window, like a titlebar
                    .child(
                        div()
                            .id("titlebar-drag")
                            .flex_1()
                            .h_full()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move()
                            }),
                    ),
            )
            .child(ui::button::icon_button(
                "new-query",
                Icon::new(AssetIcon::Plus)
                    .with_size(px(16.))
                    .text_color(colors.muted_foreground)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| this.open_query(window, cx)),
            ))
            .when(!self.show_query && self.active.is_some(), |this| {
                this.child(ui::button::icon_button(
                    "props-toggle-top",
                    Self::props_glyph(self.props_open, colors.muted_foreground)
                        .into_any_element(),
                    colors,
                    cx.listener(|this, _, window, cx| this.toggle_props(window, cx)),
                ))
            })
    }

    pub(crate) fn main_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let tab = self.active.and_then(|index| self.tabs.get(index)).cloned();

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                let Some(grid) = this.grid.clone() else {
                    return;
                };
                let editor = grid
                    .read(cx)
                    .delegate()
                    .editing
                    .as_ref()
                    .map(|edit| edit.editor.clone());
                match editor {
                    Some(CellEditor::Text(input)) => {
                        this.finish_edit(&grid, &input, cx);
                    }
                    Some(_) => {
                        grid.update(cx, |table, cx| {
                            table.delegate_mut().editing = None;
                            cx.notify();
                        });
                        cx.notify();
                    }
                    None => {}
                }
            }))
            // filter bar for table tabs; the query page has its own toolbar
            .when(!self.show_query && self.filters_visible, |this| {
                this.child(self.table_filter_bar(cx))
            })
            // body
            .child(if self.show_query {
                self.query_pane(&tab, cx).into_any_element()
            } else {
                match &tab {
                    None => Self::empty_state(
                        "选择一张表",
                        match &self.error {
                            Some(message) => format!("读取失败：{message}"),
                            None => "展开左侧连接和库，点表名打开".to_string(),
                        },
                        colors,
                    )
                    .into_any_element(),
                    Some(_) if self.columns.is_empty() && self.rows.is_empty() => {
                        match &self.error {
                            Some(message) => {
                                Self::empty_state("查询失败", message.clone(), colors)
                            }
                            None => Self::empty_state(
                                "这张表是空的",
                                "再选一张表看看".to_string(),
                                colors,
                            ),
                        }
                        .into_any_element()
                    }
                    Some(_) => match self.grid.clone() {
                        Some(grid) => div()
                            .flex_1()
                            .min_h_0()
                            .child(self.table_frame(&grid, colors, cx))
                            .into_any_element(),
                        None => match &self.error {
                            Some(message) => {
                                Self::empty_state("查询失败", message.clone(), colors)
                            }
                            None => Self::empty_state(
                                "这张表是空的",
                                "再选一张表看看".to_string(),
                                colors,
                            ),
                        }
                        .into_any_element(),
                    },
                }
            })
            .when(!self.show_query && tab.is_some(), |this| {
                this.child(self.table_footer(cx))
            })
    }

    pub(crate) fn query_summary_panel(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let mono = cx.theme().mono_font_family.clone();
        let Some(info) = &self.query_info else {
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(muted)
                .child("运行查询查看摘要")
                .into_any_element();
        };
        let (status, status_color): (&str, Hsla) = match info.status {
            QueryStatus::Running => ("执行中", colors.primary),
            QueryStatus::Success => ("成功", gpui_kit::rgb(0x2F9E44).into()),
            QueryStatus::Failed => ("失败", colors.danger_foreground),
            QueryStatus::Cancelled => ("已取消", muted),
        };
        let card = |label: &'static str, value: String| {
            v_flex()
                .flex_1()
                .p_3()
                .gap_1()
                .rounded(px(8.))
                .border_1()
                .border_color(colors.border)
                .bg(colors.secondary.opacity(0.35))
                .child(
                    div()
                        .text_size(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(value),
                )
                .child(div().text_size(px(11.)).text_color(muted).child(label))
        };
        let detail = |label: &'static str, value: String, color: Hsla| {
            h_flex()
                .justify_between()
                .items_start()
                .py_1()
                .border_b_1()
                .border_color(colors.border.opacity(0.6))
                .child(div().text_size(px(12.)).text_color(muted).child(label))
                .child(
                    div()
                        .max_w(px(520.))
                        .text_right()
                        .font_family(mono.clone())
                        .text_size(px(12.))
                        .text_color(color)
                        .child(value),
                )
        };

        v_flex()
            .id("query-summary")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .gap_3()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("查询摘要"),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(card("返回行数", info.rows.to_string()))
                    .child(card("列数", info.columns.to_string()))
                    .child(card(
                        "耗时",
                        info.elapsed_ms
                            .map(|ms| format!("{ms} ms"))
                            .unwrap_or_else(|| "—".to_string()),
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(detail(
                        "查询目标",
                        format!(
                            "{} / {} / {}",
                            info.connection, info.database, info.schema
                        ),
                        colors.foreground,
                    ))
                    .child(detail("状态", status.to_string(), status_color)),
            )
            .when_some(info.error.clone(), |this, error| {
                this.child(
                    div()
                        .p_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(colors.danger_foreground.opacity(0.35))
                        .bg(colors.danger_foreground.opacity(0.06))
                        .text_size(px(12.))
                        .text_color(colors.danger_foreground)
                        .child(error),
                )
            })
            .into_any_element()
    }

    pub(crate) fn query_info_panel(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let mono = cx.theme().mono_font_family.clone();
        let Some(info) = &self.query_info else {
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(muted)
                .child("运行查询查看信息")
                .into_any_element();
        };
        let (status, status_color): (&str, Hsla) = match info.status {
            QueryStatus::Running => ("执行中", colors.primary),
            QueryStatus::Success => ("成功", gpui_kit::rgb(0x2F9E44).into()),
            QueryStatus::Failed => ("失败", colors.danger_foreground),
            QueryStatus::Cancelled => ("已取消", muted),
        };
        let _detail = |label: &'static str, value: String, color: Hsla| {
            h_flex()
                .justify_between()
                .items_start()
                .py_1()
                .border_b_1()
                .border_color(colors.border.opacity(0.6))
                .child(div().text_size(px(12.)).text_color(muted).child(label))
                .child(
                    div()
                        .max_w(px(560.))
                        .text_right()
                        .font_family(mono.clone())
                        .text_size(px(12.))
                        .text_color(color)
                        .child(value),
                )
        };

        v_flex()
            .id("query-info")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_3()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_size(px(12.))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(status_color)
                            .child(status),
                    )
                    .child(div().text_color(colors.border).child("·"))
                    .child(
                        div()
                            .font_family(mono.clone())
                            .text_color(colors.foreground)
                            .child(format!(
                                "{} / {} / {}",
                                info.connection, info.database, info.schema
                            )),
                    )
                    .child(div().text_color(colors.border).child("·"))
                    .child(div().text_color(muted).child(
                        info.elapsed_ms
                            .map(|ms| format!("{ms} ms"))
                            .unwrap_or_else(|| "—".to_string()),
                    ))
                    .child(div().text_color(colors.border).child("·"))
                    .child(
                        div()
                            .text_color(muted)
                            .child(format!("{} 行 / {} 列", info.rows, info.columns)),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted)
                    .child("执行的 SQL"),
            )
            .child(
                div()
                    .p_3()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.secondary.opacity(0.3))
                    .font_family(mono)
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(colors.foreground)
                    .child(info.sql.clone()),
            )
            .when_some(info.error.clone(), |this, error| {
                this.child(
                    div()
                        .p_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(colors.danger_foreground.opacity(0.35))
                        .bg(colors.danger_foreground.opacity(0.06))
                        .text_size(px(12.))
                        .text_color(colors.danger_foreground)
                        .child(error),
                )
            })
            .into_any_element()
    }

    /// One tab in the strip: icon, title, optional close button.
    pub(crate) fn tab_chip(
        &self,
        id: String,
        icon: AssetIcon,
        title: String,
        is_active: bool,
        close: Option<CloseTarget>,
        colors: gpui_kit::component::ThemeColor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let muted = colors.muted_foreground;
        let weak = cx.entity().downgrade();
        let mut chip = h_flex()
            .id(SharedString::from(id.clone()))
            .h(px(32.))
            .px_2()
            .gap_1()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .text_size(px(13.))
            .when(is_active, |this| {
                this.bg(colors.secondary)
                    .text_color(colors.foreground)
                    .border_1()
                    .border_color(colors.border)
            })
            .when(!is_active, |this| {
                this.text_color(muted)
                    .hover(|style| style.bg(colors.secondary))
            })
            .child(Icon::new(icon).with_size(px(16.)))
            .child(div().max_w(px(180.)).truncate().child(title));
        match close {
            Some(target) => {
                chip = chip
                    .child(
                        div()
                            .id(SharedString::from(format!("close-{id}")))
                            .size(px(18.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .text_color(muted)
                            .hover(|style| {
                                style
                                    .bg(colors.secondary_active)
                                    .text_color(colors.foreground)
                            })
                            .child(Icon::new(AssetIcon::X).with_size(px(14.)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                match target {
                                    CloseTarget::Table(index) => this.close_tab(index, window, cx),
                                    CloseTarget::Query(index) => this.close_query(index, window, cx),
                                }
                            })),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| match target {
                        CloseTarget::Table(index) => {
                            this.close_query_menus();
                            this.show_query = false;
                            this.active = Some(index);
                            this.clear_table_filters(window, cx);
                            this.load_active(window, cx);
                        }
                        CloseTarget::Query(index) => {
                            this.cancel_running_query();
                            this.hide_props_now(cx);
                            this.close_query_menus();
                            this.show_query = true;
                            this.active_query = Some(index);
                            cx.notify();
                        }
                    }));
            }
            None => {
                chip = chip.on_click(cx.listener(|this, _, _, cx| {
                    this.close_query_menus();
                    this.show_query = true;
                    cx.notify();
                }));
            }
        }
        chip.context_menu(move |menu, _, cx| {
            let mut menu = app_menu(menu, cx);
            if let Some(target) = close {
                let weak_close = weak.clone();
                menu = menu.item(PopupMenuItem::new("关闭").on_click(move |_, window, cx| {
                    let _ = weak_close.update(cx, |this, cx| match target {
                        CloseTarget::Table(index) => this.close_tab(index, window, cx),
                        CloseTarget::Query(index) => this.close_query(index, window, cx),
                    });
                }));
                let weak_others = weak.clone();
                menu = menu.item(
                    PopupMenuItem::new("关闭其他选项卡").on_click(move |_, window, cx| {
                        let _ = weak_others.update(cx, |this, cx| {
                            this.close_others(target, window, cx)
                        });
                    }),
                );
                let weak_right = weak.clone();
                menu = menu.item(
                    PopupMenuItem::new("关闭右侧的选项卡").on_click(move |_, window, cx| {
                        let _ = weak_right.update(cx, |this, cx| {
                            this.close_right(target, window, cx)
                        });
                    }),
                );
            }
            let weak_all = weak.clone();
            menu = menu
                .item(PopupMenuItem::new("全部关闭").on_click(move |_, _, cx| {
                    let _ = weak_all.update(cx, |this, cx| this.close_all_tabs(cx));
                }))
                .separator();
            let weak_move = weak.clone();
            menu = menu
                .item(
                    PopupMenuItem::new("将选项卡移动到新窗口").on_click(move |_, _, cx| {
                        let _ = weak_move
                            .update(cx, |this, cx| this.show_notice("移动选项卡未完成", cx));
                    }),
                )
                .separator();
            let weak_fav = weak.clone();
            menu = menu.item(
                PopupMenuItem::new("添加到收藏夹").on_click(move |_, _, cx| {
                    let _ =
                        weak_fav.update(cx, |this, cx| this.show_notice("收藏夹未完成", cx));
                }),
            );
            if let Some(target) = close {
                let weak_reveal = weak.clone();
                menu = menu.item(
                    PopupMenuItem::new("在导航窗格中显示").on_click(move |_, window, cx| {
                        let _ = weak_reveal.update(cx, |this, cx| {
                            this.reveal_in_tree(target, window, cx)
                        });
                    }),
                );
            }
            menu
        })
    }
}
