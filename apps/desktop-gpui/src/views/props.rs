use crate::*;

impl Omdbx {
    pub(crate) fn filter_slot(
        &self,
        field: SuggestField,
        label: &'static str,
        color: Hsla,
        grow: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (state, value) = match field {
            SuggestField::Where => (&self.table_where, self.where_value.as_str()),
            SuggestField::Order => (&self.table_order, self.order_value.as_str()),
        };
        h_flex()
            .relative()
            .flex_basis(gpui_kit::relative(0.))
            .flex_grow(grow)
            .min_w_0()
            .child(Self::filter_field(
                state,
                value,
                label,
                color,
                cx.theme().mono_font_family.clone(),
            ))
            .when_some(self.suggest_menu(field, cx), |this, menu| this.child(menu))
    }

    pub(crate) fn table_props_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let mono = cx.theme().mono_font_family.clone();
        let badge = |text: &'static str, color: Hsla| {
            div()
                .px(px(5.))
                .h(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .rounded(px(4.))
                .bg(color.opacity(0.12))
                .text_size(px(9.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(text)
        };

        let tab = |tab: PropsTab, label: &'static str, count: Option<usize>| {
            let active = self.props_tab == tab;
            let label_color = if active { colors.foreground } else { muted };
            v_flex()
                .id(format!("props-tab-{label}"))
                .w(px(56.))
                .flex_none()
                .h_full()
                .items_center()
                .justify_center()
                .border_b_2()
                .border_color(if active {
                    colors.foreground
                } else {
                    gpui_kit::transparent_black()
                })
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.props_tab = tab;
                    cx.notify();
                }))
                .child(
                    h_flex()
                        .gap(px(2.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(if active {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(label_color)
                                .child(label),
                        )
                        .when_some(count, |this, count| {
                            this.child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(muted)
                                    .child(count.to_string()),
                            )
                        }),
                )
        };

        let empty = |text: &'static str| {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(muted)
                .child(text)
        };

        let content = if self.props_loading {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(muted)
                .child("加载中…")
                .into_any_element()
        } else if let Some(error) = &self.props_error {
            v_flex()
                .flex_1()
                .p_3()
                .text_size(px(12.))
                .text_color(colors.danger_foreground)
                .child(error.clone())
                .into_any_element()
        } else {
            match self.props_tab {
                PropsTab::Ddl => match &self.props_editor {
                    Some(editor) => div()
                        .flex_1()
                        .min_h_0()
                        .px_1()
                        .pb_1()
                        .child(
                            Editor::new(editor)
                                .readonly(true)
                                .appearance(false)
                                .bordered(false)
                                .with_size(gpui_kit::component::Size::XSmall)
                                .h_full(),
                        )
                        .into_any_element(),
                    None => empty("暂无 DDL").into_any_element(),
                },
                PropsTab::Fields => {
                    if self.props.columns.is_empty() {
                        empty("暂无字段").into_any_element()
                    } else {
                        v_flex()
                            .id("props-fields")
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_x_hidden()
                            .overflow_y_scroll()
                            .p_2()
                            .gap_1()
                            .children(self.props.columns.iter().map(|column| {
                                v_flex()
                                    .py_1()
                                    .gap(px(2.))
                                    .border_b_1()
                                    .border_color(colors.border.opacity(0.6))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                div()
                                                    .font_family(mono.clone())
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(column.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .font_family(mono.clone())
                                                    .text_size(px(10.5))
                                                    .text_color(muted)
                                                    .child(column.data_type.clone()),
                                            )
                                            .when(column.is_primary_key, |this| {
                                                this.child(badge("PK", colors.primary))
                                            })
                                            .when(
                                                !column.is_nullable && !column.is_primary_key,
                                                |this| this.child(badge("NOT NULL", muted)),
                                            ),
                                    )
                                    .when_some(column.column_default.clone(), |this, value| {
                                        this.child(
                                            div()
                                                .text_size(px(10.5))
                                                .text_color(muted)
                                                .child(format!("默认值: {value}")),
                                        )
                                    })
                                    .when_some(column.extra.clone().filter(|value| !value.is_empty()), |this, value| {
                                        this.child(
                                            div()
                                                .text_size(px(10.5))
                                            .text_color(muted)
                                                .child(value),
                                        )
                                    })
                                    .when(!column.comment.is_empty(), |this| {
                                        this.child(
                                            div()
                                                .text_size(px(10.5))
                                                .text_color(muted)
                                                .child(column.comment.clone()),
                                        )
                                    })
                            }))
                            .into_any_element()
                    }
                }
                PropsTab::Indexes => {
                    if self.props.indexes.is_empty() {
                        empty("暂无索引").into_any_element()
                    } else {
                        v_flex()
                            .id("props-indexes")
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_x_hidden()
                            .overflow_y_scroll()
                            .p_2()
                            .gap_1()
                            .children(self.props.indexes.iter().map(|index| {
                                v_flex()
                                    .py_1()
                                    .gap(px(2.))
                                    .border_b_1()
                                    .border_color(colors.border.opacity(0.6))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                div()
                                                    .font_family(mono.clone())
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(index.name.clone()),
                                            )
                                            .when(index.is_primary, |this| {
                                                this.child(badge("PRIMARY", colors.primary))
                                            })
                                            .when(index.is_unique && !index.is_primary, |this| {
                                                this.child(badge(
                                                    "UNIQUE",
                                                    gpui_kit::rgb(0x2F9E44).into(),
                                                ))
                                            })
                                            .when_some(index.index_type.clone(), |this, value| {
                                                this.child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(muted)
                                                        .child(value),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .font_family(mono.clone())
                                            .text_size(px(10.5))
                                            .text_color(muted)
                                            .child(index.columns.join(", ")),
                                    )
                                    .when_some(
                                        index
                                            .included_columns
                                            .clone()
                                            .filter(|columns| !columns.is_empty()),
                                        |this, columns| {
                                            this.child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(muted)
                                                    .child(format!("包含: {}", columns.join(", "))),
                                            )
                                        },
                                    )
                                    .when_some(index.comment.clone(), |this, value| {
                                        this.child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(muted)
                                                .child(value),
                                        )
                                    })
                            }))
                            .into_any_element()
                    }
                }
                PropsTab::ForeignKeys => {
                    if self.props.foreign_keys.is_empty() {
                        empty("暂无外键").into_any_element()
                    } else {
                        v_flex()
                            .id("props-foreign-keys")
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_x_hidden()
                            .overflow_y_scroll()
                            .p_2()
                            .gap_1()
                            .children(self.props.foreign_keys.iter().map(|key| {
                                let target = match &key.ref_schema {
                                    Some(schema) => format!(
                                        "{}.{}.{}",
                                        schema, key.ref_table, key.ref_column
                                    ),
                                    None => format!("{}.{}", key.ref_table, key.ref_column),
                                };
                                v_flex()
                                    .py_1()
                                    .gap(px(2.))
                                    .border_b_1()
                                    .border_color(colors.border.opacity(0.6))
                                    .child(
                                        div()
                                            .font_family(mono.clone())
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(key.name.clone()),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .text_size(px(10.5))
                                            .text_color(muted)
                                            .child(key.column.clone())
                                            .child("→")
                                            .child(target),
                                    )
                                    .when_some(key.on_update.clone(), |this, value| {
                                        this.child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(muted)
                                                .child(format!("ON UPDATE {value}")),
                                        )
                                    })
                                    .when_some(key.on_delete.clone(), |this, value| {
                                        this.child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(muted)
                                                .child(format!("ON DELETE {value}")),
                                        )
                                    })
                            }))
                            .into_any_element()
                    }
                }
                PropsTab::Triggers => {
                    if self.props.triggers.is_empty() {
                        empty("暂无触发器").into_any_element()
                    } else {
                        v_flex()
                            .id("props-triggers")
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_x_hidden()
                            .overflow_y_scroll()
                            .p_2()
                            .gap_1()
                            .children(self.props.triggers.iter().map(|trigger| {
                                v_flex()
                                    .py_1()
                                    .gap(px(2.))
                                    .border_b_1()
                                    .border_color(colors.border.opacity(0.6))
                                    .child(
                                        div()
                                            .font_family(mono.clone())
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(trigger.name.clone()),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .text_size(px(10.5))
                                            .text_color(muted)
                                            .child(format!(
                                                "{} {}",
                                                trigger.timing, trigger.event
                                            ))
                                            .when_some(trigger.level.clone(), |this, value| {
                                                this.child(value)
                                            })
                                            .when_some(trigger.enabled, |this, enabled| {
                                                this.child(if enabled {
                                                    "已启用"
                                                } else {
                                                    "已停用"
                                                })
                                            })
                                            .when_some(trigger.language.clone(), |this, value| {
                                                this.child(format!("LANGUAGE {value}"))
                                            }),
                                    )
                                    .when_some(trigger.condition.clone(), |this, value| {
                                        this.child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(muted)
                                                .child(format!("WHEN {value}")),
                                        )
                                    })
                                    .when_some(
                                        trigger
                                            .statement
                                            .clone()
                                            .filter(|value| !value.trim().is_empty()),
                                        |this, statement| {
                                            this.child(
                                                div()
                                                    .mt_1()
                                                    .p_1()
                                                    .rounded(px(4.))
                                                    .bg(colors.secondary)
                                                    .font_family(mono.clone())
                                                    .text_size(px(10.))
                                                    .text_color(colors.foreground)
                                                    .child(statement),
                                            )
                                        },
                                    )
                                    .when(!trigger.comment.clone().unwrap_or_default().is_empty(), |this| {
                                        this.child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(muted)
                                                .child(trigger.comment.clone().unwrap_or_default()),
                                        )
                                    })
                            }))
                            .into_any_element()
                    }
                }
            }
        };

        v_flex()
            .size_full()
            .min_w_0()
            .child({
                let loaded = !self.props_loading && self.props_error.is_none();
                h_flex()
                    .id("props-tab-scroll")
                    .min_w_0()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .gap_1()
                    .items_center()
                    .overflow_x_scroll()
                    .track_scroll(&self.props_tab_scroll)
                    .border_b_1()
                    .border_color(colors.border)
                    .child(tab(PropsTab::Ddl, "DDL", None))
                    .child(tab(
                        PropsTab::Fields,
                        "字段",
                        loaded.then_some(self.props.columns.len()),
                    ))
                    .child(tab(
                        PropsTab::Indexes,
                        "索引",
                        loaded.then_some(self.props.indexes.len()),
                    ))
                    .child(tab(
                        PropsTab::ForeignKeys,
                        "外键",
                        loaded.then_some(self.props.foreign_keys.len()),
                    ))
                    .child(tab(
                        PropsTab::Triggers,
                        "触发器",
                        loaded.then_some(self.props.triggers.len()),
                    ))
            })
            .child(content)
    }

    /// WHERE / ORDER BY input above a table tab, plus copy and refresh.
    pub(crate) fn table_filter_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let divider = || {
            div()
                .w(px(1.))
                .h(px(16.))
                .flex_none()
                .bg(colors.border)
        };

        h_flex()
            .h(px(36.))
            .flex_none()
            .px_2()
            .gap_2()
            .items_center()
            .border_b_1()
            .border_color(colors.border)
            .on_action(cx.listener(|this, _: &ConfirmTab, window, cx| {
                if !this.handle_suggestion_key("tab", window, cx) {
                    window.focus_next(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ConfirmEnter, window, cx| {
                this.handle_suggestion_key("enter", window, cx);
            }))
            .on_action(cx.listener(|this, _: &SuggestUp, window, cx| {
                if !this.handle_suggestion_key("up", window, cx) {
                    window.dispatch_action(Box::new(MoveUp), cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SuggestDown, window, cx| {
                if !this.handle_suggestion_key("down", window, cx) {
                    window.dispatch_action(Box::new(MoveDown), cx);
                }
            }))
            .child({
                let bounds = self.filter_bounds.clone();
                let split = self.filter_split;
                h_flex()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .on_prepaint(move |value, _, _| bounds.set(value))
                    .child(self.filter_slot(
                        SuggestField::Where,
                        "WHERE",
                        gpui_kit::rgb(0x2F6BFF).into(),
                        split,
                        cx,
                    ))
                    .child(
                        div()
                            .id("filter-split")
                            .w(px(9.))
                            .h_full()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_col_resize()
                            .occlude()
                            .on_drag(FilterDividerDrag, |drag, _, _, cx| {
                                cx.stop_propagation();
                                cx.new(|_| drag.clone())
                            })
                            .on_drag_move(cx.listener(
                                |this,
                                 e: &gpui_kit::DragMoveEvent<FilterDividerDrag>,
                                 _,
                                 cx| {
                                    let bounds = this.filter_bounds.get();
                                    if bounds.size.width > px(0.) {
                                        let ratio = ((e.event.position.x - bounds.origin.x)
                                            / bounds.size.width)
                                            .clamp(0.2, 0.8);
                                        this.filter_split = ratio;
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(div().w(px(1.)).h(px(16.)).bg(colors.border)),
                    )
                    .child(self.filter_slot(
                        SuggestField::Order,
                        "ORDER BY",
                        gpui_kit::rgb(0xE8722C).into(),
                        1.0 - split,
                        cx,
                    ))
            })
            .child(divider())
            .child(ui::button::icon_button(
                "table-copy",
                Icon::new(AssetIcon::Copy)
                    .with_size(px(16.))
                    .text_color(colors.foreground)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, _, cx| {
                    let mut text = this.columns.join("\t");
                    for row in &this.rows {
                        text.push('\n');
                        text.push_str(&row.join("\t"));
                    }
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
                    this.show_notice("已复制", cx);
                }),
            ))
            .child(
                Button::new("table-export")
                    .ghost()
                    .with_size(gpui_kit::component::Size::Small)
                    .tooltip("导出")
                    .icon(
                        Icon::new(AssetIcon::Download)
                            .with_size(px(16.))
                            .text_color(colors.foreground),
                    )
                    .dropdown_menu_with_anchor(gpui_kit::Anchor::TopRight, {
                        let weak = cx.entity().downgrade();
                        move |menu, window, cx| {
                            let mut menu = app_menu(menu, cx).min_w(px(190.));
                            for format in ExportFormat::ALL {
                                let weak = weak.clone();
                                menu = menu.submenu(format.label(), window, cx, move |menu, _, _cx| {
                                    let current = weak.clone();
                                    let all = weak.clone();
                                    menu.item(PopupMenuItem::new("当前页").on_click(move |_, _, cx| {
                                        let _ = current.update(cx, |this, cx| {
                                            this.export_table(format, false, cx)
                                        });
                                    }))
                                    .item(PopupMenuItem::new("全部页").on_click(move |_, _, cx| {
                                        let _ = all.update(cx, |this, cx| {
                                            this.export_table(format, true, cx)
                                        });
                                    }))
                                });
                            }
                            menu
                        }
                    }),
            )
            .child(ui::button::icon_button(
                "table-props",
                Icon::new(AssetIcon::TableProperties)
                    .with_size(px(16.))
                    .text_color(colors.foreground)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| this.toggle_props(window, cx)),
            ))
    }

    pub(crate) fn filter_field(
        state: &Entity<InputState>,
        value: &str,
        label: &'static str,
        color: Hsla,
        mono: SharedString,
    ) -> impl IntoElement {
        h_flex()
            .flex_1()
            .min_w_0()
            .h(px(26.))
            .px_1()
            .gap_1()
            .items_center()
            .child(
                div()
                    .flex_none()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(label),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h(px(26.))
                    .child(
                        h_flex()
                            .absolute()
                            .inset_0()
                            .pl(px(4.))
                            .items_center()
                            .overflow_hidden()
                            .text_size(px(13.))
                            .font_family(mono.clone())
                            .children(sql_spans(value)),
                    )
                    .child(
                        Input::new(state)
                            .hide_text(true)
                            .appearance(false)
                            .bordered(false)
                            .cleanable(true)
                            .with_size(gpui_kit::component::Size::XSmall)
                            .text_size(px(13.))
                            .text_color(gpui_kit::transparent_black())
                            .font_family(mono)
                            .line_height(px(16.))
                            .pt(px(5.))
                            .pb(px(0.))
                            .mt(px(1.)),
                    ),
            )
    }
}
