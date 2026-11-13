use crate::*;
use super::common::*;

impl Omdbx {
    pub(crate) fn query_crumb(
        id: &'static str,
        label: String,
        icon: Option<AssetIcon>,
        open: bool,
        colors: gpui_kit::component::ThemeColor,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let muted = colors.muted_foreground;
        h_flex()
            .id(id)
            .h(px(26.))
            .px_2()
            .gap_1()
            .items_center()
            .rounded(px(6.))
            .cursor_pointer()
            .text_size(px(13.))
            .when(open, |this| this.bg(colors.secondary))
            .hover(|style| style.bg(colors.sidebar_accent))
            .when_some(icon, |this, icon| {
                this.child(Icon::new(icon).with_size(px(15.)).text_color(muted))
            })
            .child(div().max_w(px(160.)).truncate().child(label))
            .child(Icon::new(IconName::ChevronDown).with_size(px(12.)).text_color(muted))
    }

    pub(crate) fn query_pane(&self, _tab: &Option<Tab>, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let (connection_id, database, schema) = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .map(|query| {
                (
                    query.connection_id.clone(),
                    query.database.clone(),
                    query.schema.clone(),
                )
            })
            .unwrap_or_default();
        let connection_name = self
            .connections
            .iter()
            .find(|item| item.id == connection_id)
            .map(|item| item.name.clone())
            .unwrap_or_else(|| "未连接".to_string());
        let databases = self
            .databases
            .get(&connection_id)
            .cloned()
            .unwrap_or_default();
        let schemas = self
            .schemas
            .get(&format!("{connection_id}|{database}"))
            .cloned()
            .unwrap_or_default();
        let weak = cx.entity().downgrade();
        let editor = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .map(|query| query.editor.clone());
        let rows = self.rows.len();
        let running = self.loading && self.running_query.is_some();
        let editable = self.edit_target().is_some();
        let dirty = editable && self.has_pending_changes();
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
        let elapsed = if self.loading {
            "执行中…".to_string()
        } else {
            self.query_ms
                .map(|ms| format!("{ms} ms"))
                .unwrap_or_else(|| "—".to_string())
        };
        let query_bounds = self.query_bounds.clone();
        let query_split = self.query_split;

        let connection_menu = Popover::new("query-connection-menu")
            .open(self.query_connection_open)
            .appearance(false)
            .offset(px(4.))
            .on_open_change({
                let weak = weak.clone();
                move |open, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.query_connection_open = *open;
                        if *open {
                            this.query_database_open = false;
                            this.query_schema_open = false;
                        }
                        cx.notify();
                    });
                }
            })
            .trigger(MenuTrigger(Self::query_crumb(
                "query-connection-trigger",
                connection_name.clone(),
                Some(AssetIcon::Database),
                self.query_connection_open,
                colors,
            )))
            .content({
                let weak = weak.clone();
                let connections = self.connections.clone();
                let selected_id = connection_id.clone();
                move |_, _, cx| {
                    let popover = cx.entity().downgrade();
                    let mut list = v_flex()
                        .w(px(232.))
                        .p_1()
                        .gap(px(1.))
                        .rounded(px(8.))
                        .bg(colors.background)
                        .border_1()
                        .border_color(colors.border)
                        .shadow_lg()
                        .occlude()
                        .child(query_menu_caption("连接", colors));
                    if connections.is_empty() {
                        list = list.child(
                            div()
                                .px_2()
                                .py_2()
                                .text_size(px(12.))
                                .text_color(colors.muted_foreground)
                                .child("暂无连接"),
                        );
                    }
                    for connection in &connections {
                        let id = connection.id.clone();
                        let selected = id == selected_id;
                        let weak = weak.clone();
                        let popover = popover.clone();
                        list = list.child(query_menu_row(
                            format!("query-connection-{id}"),
                            connection.name.clone(),
                            selected,
                            Some(AssetIcon::Database),
                            colors,
                            move |_, window, cx| {
                                let _ = popover
                                    .update(cx, |state, cx| state.dismiss(window, cx));
                                let _ = weak.update(cx, |this, cx| {
                                    this.set_query_connection(&id, cx)
                                });
                            },
                        ));
                    }
                    list
                }
            });

        let database_menu = Popover::new("query-database-menu")
            .open(self.query_database_open)
            .appearance(false)
            .offset(px(4.))
            .on_open_change({
                let weak = weak.clone();
                move |open, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.query_database_open = *open;
                        if *open {
                            this.query_connection_open = false;
                            this.query_schema_open = false;
                        }
                        cx.notify();
                    });
                }
            })
            .trigger(MenuTrigger(Self::query_crumb(
                "query-database-trigger",
                if database.is_empty() {
                    "选择数据库".to_string()
                } else {
                    database.clone()
                },
                None,
                self.query_database_open,
                colors,
            )))
            .content({
                let weak = weak.clone();
                let databases = databases.clone();
                let selected = database.clone();
                move |_, _, cx| {
                    let popover = cx.entity().downgrade();
                    let mut list = v_flex()
                        .w(px(232.))
                        .p_1()
                        .gap(px(1.))
                        .rounded(px(8.))
                        .bg(colors.background)
                        .border_1()
                        .border_color(colors.border)
                        .shadow_lg()
                        .occlude()
                        .child(query_menu_caption("数据库", colors));
                    if databases.is_empty() {
                        list = list.child(
                            div()
                                .px_2()
                                .py_2()
                                .text_size(px(12.))
                                .text_color(colors.muted_foreground)
                                .child("暂无数据库"),
                        );
                    }
                    for name in &databases {
                        let value = name.clone();
                        let is_selected = value == selected;
                        let weak = weak.clone();
                        let popover = popover.clone();
                        list = list.child(query_menu_row(
                            format!("query-database-{value}"),
                            value.clone(),
                            is_selected,
                            None,
                            colors,
                            move |_, window, cx| {
                                let _ = popover
                                    .update(cx, |state, cx| state.dismiss(window, cx));
                                let _ = weak.update(cx, |this, cx| {
                                    this.set_query_database(&value, cx)
                                });
                            },
                        ));
                    }
                    list
                }
            });

        let schema_menu = Popover::new("query-schema-menu")
            .open(self.query_schema_open)
            .appearance(false)
            .offset(px(4.))
            .on_open_change({
                let weak = weak.clone();
                move |open, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.query_schema_open = *open;
                        if *open {
                            this.query_connection_open = false;
                            this.query_database_open = false;
                        }
                        cx.notify();
                    });
                }
            })
            .trigger(MenuTrigger(Self::query_crumb(
                "query-schema-trigger",
                if schema.is_empty() {
                    "选择 Schema".to_string()
                } else {
                    schema.clone()
                },
                None,
                self.query_schema_open,
                colors,
            )))
            .content({
                let weak = weak.clone();
                let schemas = schemas.clone();
                let selected = schema.clone();
                move |_, _, cx| {
                    let popover = cx.entity().downgrade();
                    let mut list = v_flex()
                        .w(px(212.))
                        .p_1()
                        .gap(px(1.))
                        .rounded(px(8.))
                        .bg(colors.background)
                        .border_1()
                        .border_color(colors.border)
                        .shadow_lg()
                        .occlude()
                        .child(query_menu_caption("Schema", colors));
                    if schemas.is_empty() {
                        list = list.child(
                            div()
                                .px_2()
                                .py_2()
                                .text_size(px(12.))
                                .text_color(colors.muted_foreground)
                                .child("暂无 Schema"),
                        );
                    }
                    for name in &schemas {
                        let value = name.clone();
                        let is_selected = value == selected;
                        let weak = weak.clone();
                        let popover = popover.clone();
                        list = list.child(query_menu_row(
                            format!("query-schema-{value}"),
                            value.clone(),
                            is_selected,
                            None,
                            colors,
                            move |_, window, cx| {
                                let _ = popover
                                    .update(cx, |state, cx| state.dismiss(window, cx));
                                let _ = weak.update(cx, |this, cx| {
                                    this.set_query_schema(&value, cx)
                                });
                            },
                        ));
                    }
                    list
                }
            });

        let result_tab = |tab: ResultTab, label: &'static str| {
            let active = self.result_tab == tab;
            let fg = if active { colors.foreground } else { muted };
            v_flex()
                .id(format!("query-result-tab-{label}"))
                .h_full()
                .justify_center()
                .border_b_2()
                .border_color(if active {
                    colors.foreground
                } else {
                    gpui_kit::transparent_black()
                })
                .px_2()
                .cursor_pointer()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(if active {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(fg)
                                .child(label),
                        )
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.result_tab = tab;
                    cx.notify();
                }))
        };

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .on_prepaint(move |bounds, _, _| query_bounds.set(bounds))
            // connection, database and schema pickers, then the actions
            .child(
                h_flex()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .gap_1()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(connection_menu)
                    .child(
                        div()
                            .px(px(1.))
                            .text_size(px(12.))
                            .text_color(muted.opacity(0.55))
                            .child("/"),
                    )
                    .child(database_menu)
                    .child(
                        div()
                            .px(px(1.))
                            .text_size(px(12.))
                            .text_color(muted.opacity(0.55))
                            .child("/"),
                    )
                    .child(schema_menu)
                    .child(div().flex_1())
                    .child(ui::button::icon_button(
                        "format-sql",
                        Icon::new(AssetIcon::Wand)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| this.format_query_sql(window, cx)),
                    ))
                    .child(
                        Button::new("explain-query")
                            .ghost()
                            .small()
                            .tooltip("执行计划")
                            .icon(
                                Icon::new(AssetIcon::Workflow)
                                    .with_size(px(16.))
                                    .text_color(muted),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.explain_query(window, cx)
                            })),
                    )
                    .child(
                        Button::new("run-query")
                            .primary()
                            .small()
                            .tooltip("运行")
                            .icon(Icon::new(AssetIcon::Play).with_size(px(17.)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.run_query(window, cx)
                            })),
                    )
                    .child(ui::button::action_button(
                        "query-stop",
                        div()
                            .w(px(11.))
                            .h(px(11.))
                            .rounded(px(2.))
                            .bg(if running { colors.foreground } else { muted })
                            .into_any_element(),
                        running,
                        colors,
                        cx.listener(|this, _, _, cx| this.stop_query(cx)),
                    )),
            )
            // editor
            .child(
                v_flex()
                    .h(gpui_kit::relative(query_split))
                    .flex_none()
                    .min_h_0()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(13.))
                    .child(match editor {
                        Some(state) => Editor::new(&state)
                            .appearance(false)
                            .bordered(false)
                            .with_size(gpui_kit::component::Size::XSmall)
                            .h(gpui_kit::relative(1.))
                            .into_any_element(),
                        None => div().into_any_element(),
                    }),
            )
            // draggable divider between the editor and the result pane
            .child(
                div()
                    .id("query-split")
                    .h(px(6.))
                    .w_full()
                    .flex_none()
                    .cursor_row_resize()
                    .occlude()
                    .on_drag(QueryDividerDrag, |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(cx.listener(
                        |this,
                         e: &gpui_kit::DragMoveEvent<QueryDividerDrag>,
                         _,
                         cx| {
                            let bounds = this.query_bounds.get();
                            if bounds.size.height > px(0.) {
                                let ratio = ((e.event.position.y - bounds.origin.y)
                                    / bounds.size.height)
                                    .clamp(0.12, 0.82);
                                this.query_split = ratio;
                                cx.notify();
                            }
                        },
                    ))
                    .child(div().h(px(1.)).w_full().bg(colors.border)),
            )
            // result header
            .child(
                h_flex()
                    .h(px(36.))
                    .flex_none()
                    .px_3()
                    .gap_1()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(result_tab(ResultTab::Results, "结果"))
                    .child(result_tab(ResultTab::Summary, "摘要"))
                    .child(result_tab(ResultTab::Info, "信息"))
                    .child(div().flex_1())
                    .child(ui::button::icon_button(
                        "rerun-query",
                        Icon::new(AssetIcon::RotateCw)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| this.run_query(window, cx)),
                    ))
                    .child(
                        Button::new("export-results")
                            .ghost()
                            .small()
                            .tooltip("导出")
                            .icon(
                                Icon::new(AssetIcon::Download)
                                    .with_size(px(16.))
                                    .text_color(muted),
                            )
                            .dropdown_menu_with_anchor(gpui_kit::Anchor::TopRight, {
                                let weak = cx.entity().downgrade();
                                move |menu, _, cx| {
                                    let mut menu = app_menu(menu, cx).min_w(px(180.));
                                    for format in ExportFormat::ALL {
                                        let weak = weak.clone();
                                        menu = menu.item(
                                            PopupMenuItem::new(format.label()).on_click(
                                                move |_, _, cx| {
                                                    let _ = weak.update(cx, |this, cx| {
                                                        this.export_query(format, cx)
                                                    });
                                                },
                                            ),
                                        );
                                    }
                                    menu
                                }
                            }),
                    ),
            )
            // result body
            .child(match self.result_tab {
                ResultTab::Results => match self.grid.clone() {
                    Some(grid) => div()
                        .flex_1()
                        .min_h_0()
                        .child(self.table_frame(&grid, colors, cx))
                        .into_any_element(),
                    None => v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(muted)
                                .child(match &self.error {
                                    Some(message) => message.clone(),
                                    None => "运行查询查看结果".to_string(),
                                }),
                        )
                        .into_any_element(),
                },
                ResultTab::Summary => self.query_summary_panel(cx).into_any_element(),
                ResultTab::Info => self.query_info_panel(cx).into_any_element(),
            })
            // status bar
            .child(
                h_flex()
                    .h(px(32.))
                    .flex_none()
                    .px_3()
                    .gap_1()
                    .items_center()
                    .border_t_1()
                    .border_color(colors.border)
                    .text_size(px(11.))
                    .text_color(muted)
                    .child(ui::button::action_button(
                        "query-row-add",
                        Icon::new(AssetIcon::Plus)
                            .with_size(px(15.))
                            .text_color(muted)
                            .into_any_element(),
                        editable,
                        colors,
                        cx.listener(|this, _, _, cx| this.add_row(cx)),
                    ))
                    .child(ui::button::action_button(
                        "query-row-remove",
                        Icon::new(AssetIcon::Minus)
                            .with_size(px(15.))
                            .text_color(muted)
                            .into_any_element(),
                        editable,
                        colors,
                        cx.listener(|this, _, _, cx| this.delete_selected_row(cx)),
                    ))
                    .child(ui::button::action_button(
                        "query-row-save",
                        Icon::new(AssetIcon::Check)
                            .with_size(px(15.))
                            .text_color(save_color)
                            .into_any_element(),
                        dirty,
                        colors,
                        cx.listener(|this, _, window, cx| this.save_pending_edits(window, cx)),
                    ))
                    .child(ui::button::action_button(
                        "query-row-cancel",
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
                        "query-reload",
                        Icon::new(AssetIcon::RotateCw)
                            .with_size(px(15.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| this.run_query(window, cx)),
                    ))
                    .child(ui::button::action_button(
                        "query-stop",
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
                    .child(
                        h_flex()
                            .ml_2()
                            .gap_2()
                            .items_center()
                            .child(format!("{} 行", rows))
                            .child(div().text_color(colors.border).child("|"))
                            .child(div().child(elapsed)),
                    )
                    .child(div().flex_1())
                    .child(div().child("第 1/1 页"))
                    .child(
                        Icon::new(AssetIcon::ChevronLeft)
                            .with_size(px(14.))
                            .text_color(muted),
                    )
                    .child(
                        Icon::new(AssetIcon::ChevronRight)
                            .with_size(px(14.))
                            .text_color(muted),
                    )
                    .child(div().child(format!("{} 行/页", ROW_LIMIT))),
            )
    }
}
