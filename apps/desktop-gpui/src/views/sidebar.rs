use crate::*;
use super::common::*;
use super::connection_menu::{connection_context_menu, ConnectionMenuState};

fn text_matches(filter: &str, text: &str) -> bool {
    filter.is_empty() || text.to_lowercase().contains(filter)
}

impl Omdbx {
    pub(crate) fn connection_row(
        &self,
        connection: &Connection,
        depth: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_open =
            self.open_connections.contains(&connection.id) || !self.filter.trim().is_empty();
        let active = is_open
            || self
                .active
                .and_then(|index| self.tabs.get(index))
                .is_some_and(|tab| tab.connection_id == connection.id);
        let key = connection
            .driver_profile
            .clone()
            .unwrap_or_else(|| connection.db_type.clone());
        let id = connection.id.clone();
        let colors = cx.theme().colors;
        let starred = self.stars.iter().any(|id| id == &connection.id);
        let color = connection.color().map(str::to_string);
        let mut row = h_flex()
            .id(SharedString::from(format!("conn-{}", connection.id)))
            .relative()
            .h(px(32.))
            .flex_none()
            .mx_2()
            .pl(px(12. + depth * 16.))
            .pr_2()
            .gap_2()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .text_size(px(13.))
            .hover(|style| style.bg(colors.sidebar_accent))
            .child(connection_icon(&key, active))
            .child(div().min_w_0().truncate().child(connection.name.clone()));
        if starred {
            row = row.child(div().flex_1()).child(
                Icon::new(AssetIcon::StarFill)
                    .with_size(px(11.))
                    .text_color(gpui_kit::rgb(0xD9B23C)),
            );
        } else {
            row = row.child(div().flex_1());
        }
        if let Some(color) = color.as_deref() {
            if color.eq_ignore_ascii_case("rainbow") {
                row = row.child(
                    h_flex()
                        .absolute()
                        .left_0()
                        .top(px(7.))
                        .bottom(px(7.))
                        .w(px(3.))
                        .rounded_full()
                        .overflow_hidden()
                        .items_start()
                        .justify_center()
                        .child(
                            v_flex()
                                .size_full()
                                .child(div().flex_1().w_full().bg(gpui_kit::rgb(0xE5534B)))
                                .child(div().flex_1().w_full().bg(gpui_kit::rgb(0xD9B23C)))
                                .child(div().flex_1().w_full().bg(gpui_kit::rgb(0x4F9D69)))
                                .child(div().flex_1().w_full().bg(gpui_kit::rgb(0x3F7FD0)))
                                .child(div().flex_1().w_full().bg(gpui_kit::rgb(0x8A6BD1))),
                        ),
                );
            } else if let Some(color) = connection_color_value(color) {
                row = row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(7.))
                        .bottom(px(7.))
                        .w(px(3.))
                        .rounded_full()
                        .bg(color),
                );
            }
        }
        let menu_state = ConnectionMenuState {
            connection: connection.clone(),
            folders: self.folders.clone(),
            stars: self.stars.clone(),
            profiles: self
                .connection_profiles
                .get(&connection.id)
                .cloned()
                .unwrap_or_default(),
            active_profile: self.active_profiles.get(&connection.id).cloned(),
            show_empty_databases: self.show_empty_databases,
        };
        let app = cx.entity().downgrade();
        row.on_click(cx.listener(move |this, _, _, cx| this.toggle_connection(&id, cx)))
            .context_menu(move |menu, window, cx| {
                connection_context_menu(menu, window, cx, app.clone(), menu_state.clone())
            })
    }

    pub(crate) fn database_row(
        &self,
        connection_id: &str,
        database: &str,
        depth: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme().colors;
        let conn_id = connection_id.to_string();
        let db_name = database.to_string();
        tree_row(
            format!("db-{connection_id}|{database}"),
            28.0 + depth * 16.,
            Icon::new(AssetIcon::Database),
            database.to_string(),
            colors,
            cx.listener(move |this, _, _, cx| this.toggle_database(&conn_id, &db_name, cx)),
        )
    }

    fn folder_row(&self, folder: &SidebarFolder, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let open = self.open_folders.contains(&folder.id);
        let count = folder
            .connections
            .iter()
            .filter(|id| self.connections.iter().any(|connection| &connection.id == *id))
            .count();
        let id = folder.id.clone();
        h_flex()
            .id(SharedString::from(format!("folder-{}", folder.id)))
            .h(px(32.))
            .flex_none()
            .mx_2()
            .pl(px(8.))
            .pr_2()
            .gap_2()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .text_size(px(13.))
            .hover(|style| style.bg(colors.sidebar_accent))
            .child(
                Icon::new(if open {
                    AssetIcon::ChevronDown
                } else {
                    AssetIcon::ChevronRight
                })
                .with_size(px(13.))
                .text_color(colors.muted_foreground),
            )
            .child(
                Icon::new(if open {
                    AssetIcon::FolderOpen
                } else {
                    AssetIcon::Folder
                })
                .with_size(px(15.))
                .text_color(colors.muted_foreground),
            )
            .child(div().min_w_0().truncate().child(folder.name.clone()))
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(colors.muted_foreground)
                    .child(count.to_string()),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.open_folders.remove(&id) {
                    this.open_folders.insert(id.clone());
                }
                cx.notify();
            }))
    }

    fn append_connection_rows(
        &self,
        connection: &Connection,
        depth: f32,
        searching: bool,
        filter: &str,
        rows: &mut Vec<gpui_kit::AnyElement>,
        entries: &mut Vec<TreeEntry>,
        cx: &mut Context<Self>,
    ) {
        let colors = cx.theme().colors;
        let conn_match = text_matches(filter, &connection.name) || text_matches(filter, &connection.db_type);
        let mut children = Vec::new();
        let mut child_entries = Vec::new();

        for database in self
            .databases
            .get(&connection.id)
            .cloned()
            .unwrap_or_default()
        {
            let key = format!("{}|{}", connection.id, database);
            if !searching
                && !self.show_empty_databases
                && self.database_has_tables.get(&key) == Some(&false)
            {
                continue;
            }
            let db_match = text_matches(filter, &database);
            let tables = self.tables.get(&key).cloned().unwrap_or_default();
            let any_table_match = tables.iter().any(|table| text_matches(filter, table));
            if searching && !conn_match && !db_match && !any_table_match {
                continue;
            }

            let db_open = self.open_databases.contains(&key) || searching;
            children.push(
                self.database_row(&connection.id, &database, depth, cx)
                    .into_any_element(),
            );
            child_entries.push(TreeEntry::Database(connection.id.clone(), database.clone()));
            if !db_open {
                continue;
            }

            for table in tables {
                if searching && !conn_match && !db_match && !text_matches(filter, &table) {
                    continue;
                }
                let conn_id = connection.id.clone();
                let db_name = database.clone();
                let table_name = table.clone();
                children.push(
                    tree_row(
                        format!("t-{}|{}|{}", connection.id, database, table),
                        44.0 + depth * 16.,
                        Icon::new(AssetIcon::Table2),
                        table.clone(),
                        colors,
                        cx.listener(move |this, _, window, cx| {
                            this.open_table(&conn_id, &db_name, &table_name, window, cx)
                        }),
                    )
                    .into_any_element(),
                );
                child_entries.push(TreeEntry::Table(
                    connection.id.clone(),
                    database.clone(),
                ));
            }
        }

        if searching && !conn_match && children.is_empty() {
            return;
        }
        rows.push(self.connection_row(connection, depth, cx).into_any_element());
        entries.push(TreeEntry::Connection(connection.id.clone()));
        if self.open_connections.contains(&connection.id) || searching {
            rows.extend(children);
            entries.extend(child_entries);
        }
    }

    /// Sidebar tree, narrowed to rows matching the search field. While a query
    /// is active the matching branches are shown expanded and every other row
    /// is dropped, so the tree reads as a flat result list.
    pub(crate) fn tree(
        &self,
        cx: &mut Context<Self>,
    ) -> (Vec<gpui_kit::AnyElement>, Vec<TreeEntry>) {
        let filter = self.filter.trim().to_lowercase();
        let searching = !filter.is_empty();
        let mut rows = Vec::new();
        let mut entries = Vec::new();

        if searching {
            for connection in &self.connections {
                self.append_connection_rows(
                    connection,
                    0.0,
                    true,
                    &filter,
                    &mut rows,
                    &mut entries,
                    cx,
                );
            }
            return (rows, entries);
        }

        let mut placed = HashSet::new();
        for folder in &self.folders {
            rows.push(self.folder_row(folder, cx).into_any_element());
            entries.push(TreeEntry::Folder(folder.id.clone()));
            if !self.open_folders.contains(&folder.id) {
                placed.extend(folder.connections.iter().cloned());
                continue;
            }
            for connection_id in &folder.connections {
                if !placed.insert(connection_id.clone()) {
                    continue;
                }
                if let Some(connection) = self
                    .connections
                    .iter()
                    .find(|connection| &connection.id == connection_id)
                {
                    self.append_connection_rows(
                        connection,
                        1.0,
                        false,
                        &filter,
                        &mut rows,
                        &mut entries,
                        cx,
                    );
                }
            }
        }
        for connection in &self.connections {
            if placed.contains(&connection.id) {
                continue;
            }
            self.append_connection_rows(
                connection,
                0.0,
                false,
                &filter,
                &mut rows,
                &mut entries,
                cx,
            );
        }
        (rows, entries)
    }

    /// Rows pinned above the scrolling tree: the open connection, plus its
    /// database when the visible list is inside that database.
    pub(crate) fn sticky_rows(
        &self,
        entries: &[TreeEntry],
        cx: &mut Context<Self>,
    ) -> Vec<gpui_kit::AnyElement> {
        if !self.filter.trim().is_empty() || self.tree_scroll.max_offset().y <= px(0.) {
            return Vec::new();
        }
        // Nothing is scrolled past the first row: keep the list unadorned.
        let top_index = self.tree_scroll.top_item();
        if top_index == 0 {
            return Vec::new();
        }
        if let Some(TreeEntry::Folder(folder_id)) = entries.get(top_index) {
            let Some(folder) = self.folders.iter().find(|folder| &folder.id == folder_id) else {
                return Vec::new();
            };
            return vec![self.folder_row(folder, cx).into_any_element()];
        }
        let (connection_id, database) = match entries.get(top_index) {
            Some(TreeEntry::Connection(connection_id)) => (connection_id, None),
            Some(TreeEntry::Database(connection_id, database)) => (connection_id, Some(database)),
            Some(TreeEntry::Table(connection_id, database)) => (connection_id, Some(database)),
            Some(TreeEntry::Folder(_)) => return Vec::new(),
            None => return Vec::new(),
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| &connection.id == connection_id)
        else {
            return Vec::new();
        };
        let depth = if self
            .folders
            .iter()
            .any(|folder| folder.connections.iter().any(|id| id == connection_id))
        {
            1.0
        } else {
            0.0
        };
        let mut rows = vec![self
            .connection_row(connection, depth, cx)
            .into_any_element()];
        if let Some(database) = database {
            rows.push(
                self.database_row(connection_id, database, depth, cx)
                    .into_any_element(),
            );
        }
        rows
    }

    pub(crate) fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        v_flex()
            .w_full()
            .h_full()
            .bg(colors.sidebar)
            .pb(px(8.))
            // transparent titlebar: the traffic lights sit over this strip, the
            // collapse toggle hugs its right edge
            .child(
                div()
                    .relative()
                    .h(px(44.))
                    .flex_none()
                    .child(TitleBar::new().h(px(44.)).bg(colors.sidebar))
                    .child(
                        h_flex()
                            .absolute()
                            .top_0()
                            .left(px(80.))
                            .h(px(44.))
                            .items_center()
                            .child(self.sidebar_toggle(cx)),
                    ),
            )
            .child(
                v_flex()
                    .flex_none()
                    .pt_2()
                    .gap(px(2.))
                    .child(tree_row(
                        "menu-add-connection".to_string(),
                        12.,
                        Icon::new(AssetIcon::Cable),
                        "新增连接".to_string(),
                        colors,
                        cx.listener(|this, _, window, cx| {
                            this.open_connection_dialog(window, cx)
                        }),
                    ))
                    .child(tree_row(
                        "menu-new-query".to_string(),
                        12.,
                        Icon::new(AssetIcon::SquarePen),
                        "新增查询".to_string(),
                        colors,
                        cx.listener(|this, _, window, cx| this.open_query(window, cx)),
                    ))
                    .child(
                        Popover::new("tools-menu")
                            .open(self.tools_open)
                            .on_open_change({
                                let weak = cx.entity().downgrade();
                                move |open, _, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        this.tools_open = *open;
                                        cx.notify();
                                    });
                                }
                            })
                            .trigger(MenuTrigger(
                                tree_row(
                                    "menu-tools".to_string(),
                                    12.,
                                    Icon::new(AssetIcon::Toolbox),
                                    "常用工具".to_string(),
                                    colors,
                                    |_, _, _| {},
                                )
                                .child(
                                    Icon::new(if self.tools_open {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .with_size(px(16.))
                                    .text_color(colors.muted_foreground),
                                ),
                            ))
                            .content({
                                let weak = cx.entity().downgrade();
                                move |_, _, _| {
                                    v_flex()
                                        .w(px(196.))
                                        .gap(px(1.))
                                        .child(tool_menu_row(
                                            "tool-transfer",
                                            AssetIcon::ArrowLeftRight,
                                            "数据传输",
                                            "数据传输未完成",
                                            weak.clone(),
                                            colors,
                                        ))
                                        .child(tool_menu_row(
                                            "tool-backup",
                                            AssetIcon::DatabaseBackup,
                                            "数据备份",
                                            "数据备份未完成",
                                            weak.clone(),
                                            colors,
                                        ))
                                        .child(tool_menu_row(
                                            "tool-compare-schema",
                                            AssetIcon::GitCompare,
                                            "比较结构",
                                            "比较结构未完成",
                                            weak.clone(),
                                            colors,
                                        ))
                                        .child(tool_menu_row(
                                            "tool-compare-data",
                                            AssetIcon::GitCompareArrows,
                                            "比较数据",
                                            "比较数据未完成",
                                            weak.clone(),
                                            colors,
                                        ))
                                        .child(tool_menu_row(
                                            "tool-mcp",
                                            AssetIcon::Server,
                                            "MCP 设置",
                                            "MCP 设置未完成",
                                            weak.clone(),
                                            colors,
                                        ))
                                }
                            }),
                    )
                    .child(self.connections_header(cx)),
            )
            .child({
                let (tree_rows, tree_entries) = self.tree(cx);
                if self.tree_row_count.get() != tree_rows.len() {
                    self.tree_row_count.set(tree_rows.len());
                    // Scroll metrics come from the previous frame's prepaint;
                    // render once more after the new rows settle so the sticky
                    // and bottom fade appear with the expand/collapse.
                    cx.spawn(async move |weak, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(16))
                            .await;
                        let _ = weak.update(cx, |_, cx| cx.notify());
                    })
                    .detach();
                }
                let sticky = self.sticky_rows(&tree_entries, cx);
                let max_offset = self.tree_scroll.max_offset().y;
                let at_bottom = -self.tree_scroll.offset().y >= max_offset - px(1.);
                div()
                    .relative()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .id("sidebar-tree")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.tree_scroll)
                            .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
                            .pb_2()
                            .children(tree_rows),
                    )
                    .when(!sticky.is_empty(), |this| {
                        this.child(
                            v_flex()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .child(
                                    v_flex()
                                        .bg(colors.sidebar)
                                        .border_b_1()
                                        .border_color(colors.border.opacity(0.4))
                                        .children(sticky),
                                )
                                .child(
                                    div().h(px(6.)).w_full().bg(gpui_kit::linear_gradient(
                                        180.,
                                        gpui_kit::linear_color_stop(
                                            gpui_kit::hsla(0., 0., 0., 0.06),
                                            0.,
                                        ),
                                        gpui_kit::linear_color_stop(
                                            gpui_kit::hsla(0., 0., 0., 0.),
                                            1.,
                                        ),
                                    )),
                                ),
                        )
                    })
                    .when(max_offset > px(0.) && !at_bottom, |this| {
                        this.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(8.))
                                .bg(gpui_kit::linear_gradient(
                                    180.,
                                    gpui_kit::linear_color_stop(
                                        gpui_kit::hsla(0., 0., 0., 0.),
                                        0.,
                                    ),
                                    gpui_kit::linear_color_stop(
                                        gpui_kit::hsla(0., 0., 0., 0.06),
                                        1.,
                                    ),
                                )),
                        )
                    })
            })
            .child(
                h_flex()
                    .h(px(32.))
                    .flex_none()
                    .px_3()
                    .items_center()
                    .border_t_1()
                    .border_color(colors.border)
                    .justify_between()
                    .child({
                        let weak = cx.entity().downgrade();
                        let language = self.language.clone();
                        Button::new("settings")
                            .ghost()
                            .with_size(gpui_kit::component::Size::Small)
                            .icon(
                                Icon::new(AssetIcon::Settings)
                                    .with_size(px(16.))
                                    .text_color(colors.muted_foreground),
                            )
                            .dropdown_menu_with_anchor(
                                gpui_kit::Anchor::BottomLeft,
                                move |menu, window, cx| {
                                    let weak = weak.clone();
                                    let language = language.clone();
                                    app_menu(menu, cx).min_w(px(180.)).submenu(
                                        "语言 / Language",
                                        window,
                                        cx,
                                        move |menu, _, _cx| {
                                            let mut menu = menu;
                                            for (code, label) in SUPPORTED_LANGUAGES {
                                                let weak = weak.clone();
                                                let selected = language == code;
                                                menu = menu.item(
                                                    PopupMenuItem::new(label)
                                                        .checked(selected)
                                                        .on_click(move |_, _, cx| {
                                                            let _ = weak.update(
                                                                cx,
                                                                |this, cx| {
                                                                    this.set_language(code, cx)
                                                                },
                                                            );
                                                        }),
                                                );
                                            }
                                            menu
                                        },
                                    )
                                },
                            )
                    })
                    // no update, no icon
                    .when_some(self.update.clone(), |this, update| {
                        let color = update.level.color();
                        this.child(ui::button::icon_button(
                            "update",
                            Icon::new(AssetIcon::CircleArrowUp)
                                .with_size(px(16.))
                                .text_color(color)
                                .into_any_element(),
                            colors,
                            cx.listener(move |this, _, _, cx| {
                                this.show_notice(
                                    &format!("新版本 {} 可用", update.version),
                                    cx,
                                )
                            }),
                        ))
                    }),
            )
    }


}
