use super::*;

impl Omdbx {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = load_language();
        gpui_kit::component::set_locale(&language);
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索连接或表"));
        let table_where = cx.new(|cx| InputState::new(window, cx));
        let table_order = cx.new(|cx| InputState::new(window, cx));
        cx.subscribe(&table_where, |this, state, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                this.where_value = state.read(cx).value().to_string();
                this.where_selected = 0;
                if !this.where_suppress_open {
                    this.where_open = true;
                }
                cx.notify();
            }
            InputEvent::Focus => {
                this.where_open = true;
                this.where_selected = 0;
                cx.notify();
            }
            InputEvent::Blur => {
                this.where_open = false;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        cx.subscribe(&table_order, |this, state, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                this.order_value = state.read(cx).value().to_string();
                this.order_selected = 0;
                if !this.order_suppress_open {
                    this.order_open = true;
                }
                cx.notify();
            }
            InputEvent::Focus => {
                this.order_open = true;
                this.order_selected = 0;
                cx.notify();
            }
            InputEvent::Blur => {
                this.order_open = false;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        cx.subscribe(&search, |this, state, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                this.filter = state.read(cx).value().to_string();
                cx.notify();
            }
            InputEvent::Blur => {
                if this.search_open {
                    this.search_open = false;
                    this.filter.clear();
                    cx.notify();
                }
            }
            _ => {}
        })
        .detach();
        let (connections, error) = match fetch_connections() {
            Ok(list) => (list, None),
            Err(message) => (Vec::new(), Some(message)),
        };
        // open the first connection straight away, like the Electron build does
        let mut open_connections = HashSet::new();
        let mut databases = HashMap::new();
        if let Some(first) = connections.first() {
            if let Ok(list) = fetch_databases(&first.id) {
                databases.insert(first.id.clone(), list);
                open_connections.insert(first.id.clone());
            }
        } else if error.is_none() {
            // no connections saved yet
        }
        let this = Self {
            connections,
            error,
            open_connections,
            databases,
            open_databases: HashSet::new(),
            tables: HashMap::new(),
            tabs: Vec::new(),
            active: None,
            columns: Vec::new(),
            rows: Vec::new(),
            loading: false,
            running_query: None,
            cancel_requested: None,
            grid: None,
            queries: Vec::new(),
            active_query: None,
            next_query: 0,
            show_query: false,
            schemas: HashMap::new(),
            result_tab: ResultTab::Results,
            query_info: None,
            query_edit: None,
            query_split: 0.52,
            query_bounds: Rc::new(Cell::new(Default::default())),
            query_connection_open: false,
            query_database_open: false,
            query_schema_open: false,
            search,
            filter: String::new(),
            sidebar_open: true,
            notice: None,
            notice_id: 0,
            tools_open: false,
            query_ms: None,
            column_meta: Vec::new(),
            show_comments: true,
            grid_show_types: true,
            grid_zoom: 1.0,
            grid_row_height: ui::tokens::ROW_HEIGHT,
            grid_viewport: Rc::new(Cell::new(0.)),
            column_vis_open: false,
            width_dialog: None,
            connection_dialog: None,
            pending_edits: HashMap::new(),
            new_rows: HashSet::new(),
            pending_deletes: HashMap::new(),
            saving: false,
            update: None,
            search_open: false,
            filters_visible: true,
            filter_split: 2.0 / 3.0,
            filter_bounds: Rc::new(Cell::new(Default::default())),
            props_open: false,
            props_anim: 0.0,
            props_anim_epoch: 0,
            props_split: 0.34,
            content_bounds: Rc::new(Cell::new(Default::default())),
            props_tab: PropsTab::Ddl,
            props_loading: false,
            props_error: None,
            props: TableProps::default(),
            props_editor: None,
            props_target: None,
            language,
            focus_handle: cx.focus_handle(),
            tree_scroll: ScrollHandle::new(),
            tab_scroll: ScrollHandle::new(),
            props_tab_scroll: ScrollHandle::new(),
            tree_row_count: std::cell::Cell::new(0),
            table_where,
            table_order,
            table_page: 1,
            table_has_more: false,
            where_value: String::new(),
            where_open: false,
            where_suppress_open: false,
            where_selected: 0,
            order_value: String::new(),
            order_open: false,
            order_suppress_open: false,
            order_selected: 0,
        };
        this.check_for_update(cx);
        this
    }

    pub(crate) fn set_language(&mut self, language: &str, cx: &mut Context<Self>) {
        if self.language == language {
            return;
        }
        self.language = language.to_string();
        gpui_kit::component::set_locale(language);
        save_language(language);
        cx.refresh_windows();
        cx.notify();
    }

    pub(crate) fn toggle_connection(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.open_connections.remove(id) {
            cx.notify();
            return;
        }
        if !self.databases.contains_key(id) {
            match fetch_databases(id) {
                Ok(list) => {
                    self.databases.insert(id.to_string(), list);
                    self.error = None;
                }
                Err(message) => {
                    self.error = Some(message);
                    cx.notify();
                    return;
                }
            }
        }
        self.open_connections.insert(id.to_string());
        cx.notify();
    }

    pub(crate) fn open_connection_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_query_menus();
        let selected = database_type("mysql");
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("搜索数据库类型")
        });
        cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
        let name = cx.new(|cx| {
            InputState::new(window, cx).placeholder("连接名称，留空则自动生成")
        });
        let host = cx.new(|cx| InputState::new(window, cx).placeholder("127.0.0.1"));
        let port = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(selected.default_port.to_string(), window, cx);
            state
        });
        let username = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(selected.default_user, window, cx);
            state
        });
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("连接密码")
        });
        let database = cx.new(|cx| InputState::new(window, cx).placeholder("可选"));
        search.update(cx, |state, cx| state.focus(window, cx));
        self.connection_dialog = Some(NewConnectionDialog {
            id: uuid::Uuid::new_v4().to_string(),
            step: ConnectionDialogStep::Pick,
            category: selected.category,
            selected,
            search,
            name,
            host,
            port,
            username,
            password,
            database,
            ssl: false,
            save_password: true,
            busy: None,
            result: None,
        });
        cx.notify();
    }

    pub(crate) fn close_connection_dialog(&mut self, cx: &mut Context<Self>) {
        self.connection_dialog = None;
        cx.notify();
    }

    pub(crate) fn set_connection_category(
        &mut self,
        category: DatabaseCategory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(dialog) = self.connection_dialog.as_mut() else {
            return;
        };
        dialog.category = category;
        dialog
            .search
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    pub(crate) fn select_connection_type(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(dialog) = self.connection_dialog.as_mut() else {
            return;
        };
        dialog.selected = database_type(key);
        dialog.result = None;
        cx.notify();
    }

    pub(crate) fn show_connection_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.connection_dialog.as_mut() else {
            return;
        };
        dialog.step = ConnectionDialogStep::Form;
        dialog.result = None;
        dialog.host.update(cx, |state, cx| state.set_value("", window, cx));
        dialog
            .port
            .update(cx, |state, cx| {
                state.set_value(dialog.selected.default_port.to_string(), window, cx)
            });
        dialog.username.update(cx, |state, cx| {
            state.set_value(dialog.selected.default_user, window, cx)
        });
        dialog
            .database
            .update(cx, |state, cx| state.set_value("", window, cx));
        dialog
            .password
            .update(cx, |state, cx| state.set_value("", window, cx));
        dialog.host.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    pub(crate) fn show_connection_types(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.connection_dialog.as_mut() else {
            return;
        };
        dialog.step = ConnectionDialogStep::Pick;
        dialog.result = None;
        dialog
            .search
            .update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    fn connection_dialog_config(
        &self,
        cx: &mut Context<Self>,
    ) -> Result<(serde_json::Value, String), String> {
        let Some(dialog) = self.connection_dialog.as_ref() else {
            return Err("连接对话框已关闭".to_string());
        };
        let host = dialog.host.read(cx).value().trim().to_string();
        if host.is_empty() {
            return Err(if dialog.selected.local_file {
                "请填写数据库文件路径".to_string()
            } else {
                "请填写主机地址".to_string()
            });
        }
        let port = if dialog.selected.local_file {
            0
        } else {
            dialog
                .port
                .read(cx)
                .value()
                .trim()
                .parse::<u16>()
                .map_err(|_| "端口必须是 0-65535 的数字".to_string())?
        };
        if !dialog.selected.local_file && port == 0 {
            return Err("端口不能为 0".to_string());
        }
        let name = dialog.name.read(cx).value().trim().to_string();
        let name = if name.is_empty() {
            format!("{} 连接", dialog.selected.label)
        } else {
            name
        };
        let username = if dialog.selected.local_file {
            String::new()
        } else {
            dialog.username.read(cx).value().trim().to_string()
        };
        let database = dialog.database.read(cx).value().trim().to_string();
        let password = dialog.password.read(cx).value().to_string();
        let mut config = json!({
            "id": dialog.id,
            "name": name,
            "db_type": dialog.selected.db_type,
            "host": host,
            "port": port,
            "username": username,
            "password": password,
            "save_password": dialog.save_password,
            "ssl": dialog.ssl,
        });
        if let Some(profile) = dialog.selected.driver_profile {
            config["driver_profile"] = json!(profile);
        }
        if !database.is_empty() {
            config["database"] = json!(database);
        }
        Ok((config, dialog.id.clone()))
    }

    pub(crate) fn test_connection_dialog(&mut self, cx: &mut Context<Self>) {
        if self
            .connection_dialog
            .as_ref()
            .and_then(|dialog| dialog.busy)
            .is_some()
        {
            return;
        }
        let config = match self.connection_dialog_config(cx) {
            Ok((config, _)) => config,
            Err(message) => {
                if let Some(dialog) = self.connection_dialog.as_mut() {
                    dialog.result = Some((false, message));
                }
                cx.notify();
                return;
            }
        };
        if let Some(dialog) = self.connection_dialog.as_mut() {
            dialog.busy = Some(ConnectionDialogBusy::Test);
            dialog.result = None;
        }
        cx.notify();
        let task = cx
            .background_executor()
            .spawn(async move { test_connection_config(&config) });
        cx.spawn(async move |weak, cx| {
            let result = task.await;
            let _ = weak.update(cx, |this, cx| {
                if let Some(dialog) = this.connection_dialog.as_mut() {
                    dialog.busy = None;
                    dialog.result = Some(match result {
                        Ok(message) => (
                            true,
                            if message.trim().is_empty() {
                                "连接成功".to_string()
                            } else {
                                message
                            },
                        ),
                        Err(message) => (false, message),
                    });
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn save_connection_dialog(&mut self, cx: &mut Context<Self>) {
        if self
            .connection_dialog
            .as_ref()
            .and_then(|dialog| dialog.busy)
            .is_some()
        {
            return;
        }
        let config = match self.connection_dialog_config(cx) {
            Ok((config, _)) => config,
            Err(message) => {
                if let Some(dialog) = self.connection_dialog.as_mut() {
                    dialog.result = Some((false, message));
                }
                cx.notify();
                return;
            }
        };
        if let Some(dialog) = self.connection_dialog.as_mut() {
            dialog.busy = Some(ConnectionDialogBusy::Save);
            dialog.result = None;
        }
        cx.notify();
        let task = cx
            .background_executor()
            .spawn(async move { save_connection_config(&config) });
        cx.spawn(async move |weak, cx| {
            let result = task.await;
            let _ = weak.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        this.connection_dialog = None;
                        this.reload(cx);
                        this.show_notice("连接已保存", cx);
                    }
                    Err(message) => {
                        if let Some(dialog) = this.connection_dialog.as_mut() {
                            dialog.busy = None;
                            dialog.result = Some((false, message));
                        }
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    pub(crate) fn toggle_database(&mut self, connection_id: &str, database: &str, cx: &mut Context<Self>) {
        let key = format!("{connection_id}|{database}");
        if self.open_databases.remove(&key) {
            cx.notify();
            return;
        }
        if !self.tables.contains_key(&key) {
            let Some(connection) = self.connections.iter().find(|item| item.id == connection_id)
            else {
                return;
            };
            let schema = connection
                .default_schema
                .clone()
                .or_else(|| {
                    fetch_schemas(connection_id, database)
                        .ok()
                        .and_then(|list| list.into_iter().next())
                })
                .unwrap_or_else(|| database.to_string());
            match fetch_tables(connection_id, database, &schema) {
                Ok(list) => {
                    self.tables.insert(key.clone(), list);
                    self.error = None;
                }
                Err(message) => {
                    self.error = Some(message);
                    cx.notify();
                    return;
                }
            }
        }
        self.open_databases.insert(key);
        cx.notify();
    }

    /// Reveal the active table in the tree: expand its connection and database,
    /// load anything missing, then scroll the row into view. No active table
    /// means no-op.
    pub(crate) fn locate_active_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_query {
            return;
        }
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            return;
        };

        // Locating reveals the full tree, so drop an active filter first.
        if self.search_open || !self.filter.is_empty() {
            self.search
                .update(cx, |state, cx| state.set_value("", window, cx));
            self.filter.clear();
            self.search_open = false;
        }

        if !self.databases.contains_key(&tab.connection_id) {
            match fetch_databases(&tab.connection_id) {
                Ok(list) => {
                    self.databases.insert(tab.connection_id.clone(), list);
                }
                Err(_) => return,
            }
        }
        self.open_connections.insert(tab.connection_id.clone());

        let key = format!("{}|{}", tab.connection_id, tab.database);
        if !self.tables.contains_key(&key) {
            let Some(connection) = self
                .connections
                .iter()
                .find(|item| item.id == tab.connection_id)
            else {
                return;
            };
            let schema = connection
                .default_schema
                .clone()
                .or_else(|| {
                    fetch_schemas(&tab.connection_id, &tab.database)
                        .ok()
                        .and_then(|list| list.into_iter().next())
                })
                .unwrap_or_else(|| tab.database.clone());
            match fetch_tables(&tab.connection_id, &tab.database, &schema) {
                Ok(list) => {
                    self.tables.insert(key.clone(), list);
                }
                Err(_) => return,
            }
        }
        self.open_databases.insert(key);

        if let Some(index) = self.tree_index_of_table(&tab) {
            self.tree_scroll.scroll_to_item(index);
        }
        cx.notify();
    }

    /// Flat index of a table row among the tree's direct children, mirroring the
    /// row order built in `tree`.
    pub(crate) fn tree_index_of_table(&self, tab: &Tab) -> Option<usize> {
        let mut index = 0;
        for connection in &self.connections {
            index += 1; // connection row
            if !self.open_connections.contains(&connection.id) {
                continue;
            }
            let Some(databases) = self.databases.get(&connection.id) else {
                continue;
            };
            for database in databases {
                let key = format!("{}|{}", connection.id, database);
                let open = self.open_databases.contains(&key);
                if connection.id == tab.connection_id && database == &tab.database {
                    if !open {
                        return None;
                    }
                    return self
                        .tables
                        .get(&key)?
                        .iter()
                        .position(|name| name == &tab.table)
                        .map(|position| index + 1 + position);
                }
                index += 1; // database row
                if open {
                    index += self.tables.get(&key).map_or(0, Vec::len);
                }
            }
        }
        None
    }

    pub(crate) fn open_table(
        &mut self,
        connection_id: &str,
        database: &str,
        table: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self.connections.iter().find(|item| item.id == connection_id).cloned()
        else {
            return;
        };
        let schema = connection
            .default_schema
            .clone()
            .unwrap_or_else(|| database.to_string());
        let key = format!("{connection_id}|{database}|{table}");
        let tab_index = if let Some(index) = self.tabs.iter().position(|tab| tab.key == key) {
            self.active = Some(index);
            index
        } else {
            self.tabs.push(Tab {
                key: key.clone(),
                title: table.to_string(),
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema,
                table: table.to_string(),
            });
            let index = self.tabs.len() - 1;
            self.active = Some(index);
            index
        };
        self.tab_scroll.scroll_to_item(tab_index);
        self.close_query_menus();
        self.clear_table_filters(window, cx);
        self.load_active(window, cx);
        self.sync_props_if_open(window, cx);
    }

    pub(crate) fn clear_table_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.table_page = 1;
        self.table_has_more = false;
        self.table_where
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.table_order
            .update(cx, |state, cx| state.set_value("", window, cx));
    }

    pub(crate) fn load_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A reload rebuilds the rows, so staged edits no longer map to them.
        if self.has_pending_changes() {
            self.discard_pending_changes(cx);
            self.show_notice("未保存的修改已丢弃", cx);
        }
        self.sync_flags(cx);
        let Some(index) = self.active else { return };
        let Some(tab) = self.tabs.get(index).cloned() else { return };
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == tab.connection_id)
            .cloned()
        else {
            return;
        };
        if let Some(previous) = self.running_query.take() {
            Self::cancel_query_id(previous);
        }
        self.loading = true;
        let execution_id = uuid::Uuid::new_v4().to_string();
        self.running_query = Some(execution_id.clone());
        self.cancel_requested = None;
        cx.notify();
        let where_clause = self.table_where.read(cx).value().trim().to_string();
        let order_clause = self.table_order.read(cx).value().trim().to_string();
        let row_offset = self.table_page.saturating_sub(1) * ROW_LIMIT;
        let grid_connection = Some(connection.clone());
        let grid_table = Some(tab.table.clone());
        let task_id = execution_id.clone();
        let task = cx.background_executor().spawn(async move {
            let data = fetch_rows(
                &connection,
                &tab.database,
                &tab.schema,
                &tab.table,
                &where_clause,
                &order_clause,
                row_offset,
                Some(&task_id),
            )?;
            let table_meta = fetch_columns(
                &tab.connection_id,
                &tab.database,
                &tab.schema,
                &tab.table,
            )
            .unwrap_or_default();
            Ok::<(TableData, Vec<ColumnMeta>), String>((data, table_meta))
        });
        cx.spawn_in(window, async move |weak, cx| {
            let result = task.await;
            let _ = weak.update_in(cx, |this, window, cx| {
                if this.running_query.as_deref() != Some(execution_id.as_str()) {
                    return;
                }
                let cancelled =
                    this.cancel_requested.as_deref() == Some(execution_id.as_str());
                this.running_query = None;
                this.cancel_requested = None;
                this.loading = false;
                match result {
                    Ok((data, table_meta)) => {
                        let meta: Vec<ColumnMeta> = data
                            .columns
                            .iter()
                            .map(|name| {
                                table_meta
                                    .iter()
                                    .find(|item| &item.name == name)
                                    .cloned()
                                    .unwrap_or_default()
                            })
                            .collect();
                        this.column_meta = meta.clone();
                        let widths = Rc::new(RefCell::new(HashMap::new()));
                        let delegate = GridDelegate {
                            columns: data.columns.clone(),
                            rows: data.rows.clone(),
                            meta,
                            show_comments: this.show_comments,
                            show_types: this.grid_show_types,
                            zoom: this.grid_zoom,
                            hidden: HashSet::new(),
                            widths: widths.clone(),
                            viewport: this.grid_viewport.clone(),
                            frozen: 0,
                            app: cx.entity().downgrade(),
                            connection: grid_connection,
                            table_name: grid_table,
                            row_offset,
                            editing: None,
                            dirty: HashSet::new(),
                            new_rows: HashSet::new(),
                            deleted_rows: HashSet::new(),
                            weak: Some(cx.entity().downgrade()),
                            editable: true,
                        };
                        this.install_grid(delegate, window, cx);
                        this.columns = data.columns;
                        this.rows = data.rows;
                        this.table_has_more = data.has_more;
                        this.error = None;
                    }
                    Err(message) => {
                        if cancelled {
                            this.show_notice("已停止查询", cx);
                        } else {
                            this.grid = None;
                            this.columns = Vec::new();
                            this.rows = Vec::new();
                            this.table_has_more = false;
                            this.error = Some(message);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn toggle_props(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.props_open {
            self.animate_props(false, cx);
        } else {
            self.open_props(window, cx);
        }
    }

    pub(crate) fn hide_props_now(&mut self, cx: &mut Context<Self>) {
        self.props_open = false;
        self.props_anim_epoch = self.props_anim_epoch.wrapping_add(1);
        self.props_anim = 0.0;
        cx.notify();
    }

    pub(crate) fn animate_props(&mut self, open: bool, cx: &mut Context<Self>) {
        self.props_open = open;
        let target = if open { 1.0 } else { 0.0 };
        if (self.props_anim - target).abs() < 0.001 {
            self.props_anim = target;
            cx.notify();
            return;
        }
        self.props_anim_epoch = self.props_anim_epoch.wrapping_add(1);
        let epoch = self.props_anim_epoch;
        let start = self.props_anim;
        cx.spawn(async move |weak, cx| {
            let steps = 12u32;
            for step in 1..=steps {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let progress = step as f32 / steps as f32;
                let eased = 1.0 - (1.0 - progress).powi(3);
                let value = start + (target - start) * eased;
                let _ = weak.update(cx, |this, cx| {
                    if this.props_anim_epoch != epoch {
                        return;
                    }
                    this.props_anim = value;
                    cx.notify();
                });
            }
            let _ = weak.update(cx, |this, cx| {
                if this.props_anim_epoch != epoch {
                    return;
                }
                this.props_anim = target;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn open_props(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            return;
        };
        if self.props_open && self.props_target.as_deref() == Some(tab.key.as_str()) {
            return;
        }
        self.props_open = true;
        self.props_tab = PropsTab::Ddl;
        self.props_loading = true;
        self.props_error = None;
        self.props = TableProps::default();
        self.props_target = Some(tab.key.clone());
        self.props_editor = None;
        self.animate_props(true, cx);
        let target = tab.key.clone();
        let connection_id = tab.connection_id.clone();
        let database = tab.database.clone();
        let schema = tab.schema.clone();
        let table = tab.table.clone();
        let task = cx.background_executor().spawn(async move {
            fetch_table_props(&connection_id, &database, &schema, &table)
        });
        cx.spawn_in(window, async move |weak, cx| {
            let result = task.await;
            let _ = weak.update_in(cx, |this, window, cx| {
                if !this.props_open || this.props_target.as_deref() != Some(target.as_str()) {
                    return;
                }
                this.props_loading = false;
                match result {
                    Ok(props) => {
                        this.props_editor = Some(cx.new(|cx| {
                            EditorState::new(window, cx)
                                .language("sql")
                                .soft_wrap(false)
                                .default_value(props.ddl.clone())
                        }));
                        this.props = props;
                        this.props_error = None;
                    }
                    Err(error) => this.props_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn sync_props_if_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.props_open {
            self.open_props(window, cx);
        }
    }

    pub(crate) fn stop_query(&mut self, cx: &mut Context<Self>) {
        let Some(execution_id) = self.running_query.clone() else {
            return;
        };
        if self.cancel_requested.as_deref() == Some(execution_id.as_str()) {
            return;
        }
        self.cancel_requested = Some(execution_id.clone());
        Self::cancel_query_id(execution_id);
        self.show_notice("正在停止查询", cx);
    }

    pub(crate) fn cancel_running_query(&mut self) {
        if let Some(execution_id) = self.running_query.take() {
            Self::cancel_query_id(execution_id);
        }
        self.cancel_requested = None;
        self.loading = false;
    }

    pub(crate) fn cancel_query_id(execution_id: String) {
        std::thread::spawn(move || {
            let _ = post_json(
                &format!("{API}/api/query/cancel"),
                &json!({ "executionId": execution_id }),
            );
        });
    }

    pub(crate) fn install_grid(
        &mut self,
        delegate: GridDelegate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let widths = delegate.widths.clone();
        let grid = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .col_resizable(true)
                .cell_selectable(true)
                .row_header(false)
        });
        self.grid = Some(grid.clone());
        cx.subscribe_in(
            &grid,
            window,
            move |this, grid, event: &TableEvent, window, cx| match event {
                TableEvent::DoubleClickedCell(row, col) => {
                    this.begin_cell_edit(*row, *col, grid, window, cx);
                }
                TableEvent::ColumnWidthsChanged(new_widths) => {
                    let zoom = grid.read(cx).delegate().zoom.max(0.1);
                    let mut map = widths.borrow_mut();
                    map.clear();
                    for (view, width) in new_widths.iter().enumerate().skip(1) {
                        if let Some(data) = grid.read(cx).delegate().view_to_data(view) {
                            map.insert(data, f32::from(*width) / zoom);
                        }
                    }
                    drop(map);
                    // Width changes (drag, fit, set) can change how much space
                    // the last column has to absorb. Rebuild once the current
                    // table update has finished to avoid re-entrant updates.
                    let grid = grid.clone();
                    cx.defer_in(window, move |_this, _window, cx| {
                        grid.update(cx, |state, cx| state.refresh(cx));
                    });
                }
                _ => {}
            },
        )
        .detach();
    }

    pub(crate) fn begin_cell_edit(
        &mut self,
        row: usize,
        col: usize,
        grid: &Entity<TableState<GridDelegate>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if col == 0 || !grid.read(cx).delegate().editable {
            return;
        }
        let Some(col) = grid.read(cx).delegate().view_to_data(col) else {
            return;
        };
        if self.pending_deletes.contains_key(&row) {
            return;
        }
        self.commit_editing(grid, cx);
        let Some(original) = self
            .rows
            .get(row)
            .and_then(|cells| cells.get(col))
            .cloned()
        else {
            return;
        };
        if self.columns.get(col).is_none() {
            return;
        }
        let column_type = self
            .column_meta
            .get(col)
            .map(|meta| meta.data_type.clone())
            .unwrap_or_default();
        let kind = temporal_kind(&column_type);

        if kind == TemporalKind::Text {
            let input = cx.new(|cx| InputState::new(window, cx));
            input.update(cx, |state, cx| {
                state.set_value(original, window, cx);
                state.select_all(window, cx);
            });
            grid.update(cx, |table, cx| {
                table.delegate_mut().editing = Some(EditingCell {
                    row,
                    col,
                    editor: CellEditor::Text(input.clone()),
                });
                cx.notify();
            });
            // The click that opened the editor continues to the table after
            // this handler, so focus the input once that event turn has finished.
            let focused = input.clone();
            window.defer(cx, move |window, cx| {
                focused.update(cx, |state, cx| state.focus(window, cx));
            });
            let grid = grid.clone();
            cx.subscribe_in(&input, window, move |this, input, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    this.finish_edit(&grid, input, cx);
                }
            })
            .detach();
            return;
        }

        if kind == TemporalKind::Time {
            let state = cx.new(|cx| TimeFieldState::new(window, cx));
            state.update(cx, |state, cx| {
                state.set_precision(TimePrecision::Second, window, cx);
                if let Some(value) = parse_time(&original) {
                    state.set_time(value, window, cx);
                }
            });
            grid.update(cx, |table, cx| {
                table.delegate_mut().editing = Some(EditingCell {
                    row,
                    col,
                    editor: CellEditor::Time(state.clone()),
                });
                cx.notify();
            });
            cx.subscribe_in(&state, window, move |this, _, event: &TimeFieldEvent, _, cx| {
                let TimeFieldEvent::Change(value) = event;
                let text = value.format("%H:%M:%S").to_string();
                this.stage_cell_value(row, col, &text, cx);
            })
            .detach();
            let state_id = state.entity_id();
            let focus = Focusable::focus_handle(&*state.read(cx), cx);
            cx.on_blur(&focus, window, move |this, _window, cx| {
                let clear = this.grid.as_ref().is_some_and(|grid| {
                    grid.read(cx)
                        .delegate()
                        .editing
                        .as_ref()
                        .is_some_and(|edit| {
                            edit.row == row
                                && edit.col == col
                                && matches!(
                                    &edit.editor,
                                    CellEditor::Time(current)
                                        if current.entity_id() == state_id
                                )
                        })
                });
                if clear {
                    if let Some(grid) = this.grid.clone() {
                        grid.update(cx, |table, cx| {
                            table.delegate_mut().editing = None;
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
            let focused = state.clone();
            window.defer(cx, move |window, cx| {
                focused.update(cx, |state, cx| state.focus(window, cx));
            });
            return;
        }

        let with_time = kind == TemporalKind::DateTime;
        let format = date_picker_format(&self.language, with_time);
        let state = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format(format);
            if with_time {
                state = state.time_precision(TimePrecision::Second);
            }
            state
        });
        if let Some(value) = parse_datetime(&original) {
            state.update(cx, |state, cx| {
                state.set_date_time(DateTime::Single(Some(value)), window, cx)
            });
        }
        grid.update(cx, |table, cx| {
            table.delegate_mut().editing = Some(EditingCell {
                row,
                col,
                editor: CellEditor::Date(state.clone()),
            });
            cx.notify();
        });
        let grid_for_events = grid.clone();
        cx.subscribe_in(&state, window, move |this, _, event: &DatePickerEvent, _, cx| {
            let DatePickerEvent::Change(value) = event;
            if let Some(value) = value.start() {
                let text = if with_time {
                    value.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    value.format("%Y-%m-%d").to_string()
                };
                this.stage_cell_value(row, col, &text, cx);
                if !with_time {
                    grid_for_events.update(cx, |table, cx| {
                        table.delegate_mut().editing = None;
                        cx.notify();
                    });
                }
            }
        })
        .detach();
        let focused = state.clone();
        window.defer(cx, move |window, cx| {
            focused.update(cx, |state, cx| {
                state.focus_handle(cx).focus(window, cx);
            });
        });
    }

    pub(crate) fn commit_editing(&mut self, grid: &Entity<TableState<GridDelegate>>, cx: &mut Context<Self>) {
        let editor = grid
            .read(cx)
            .delegate()
            .editing
            .as_ref()
            .map(|edit| edit.editor.clone());
        match editor {
            Some(CellEditor::Text(input)) => self.finish_edit(grid, &input, cx),
            Some(_) => {
                grid.update(cx, |table, cx| {
                    table.delegate_mut().editing = None;
                    cx.notify();
                });
            }
            None => {}
        }
    }

    pub(crate) fn finish_edit(
        &mut self,
        grid: &Entity<TableState<GridDelegate>>,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = ({
            let delegate = grid.read(cx).delegate();
            delegate
                .editing
                .as_ref()
                .filter(|edit| {
                    matches!(
                        &edit.editor,
                        CellEditor::Text(current) if current.entity_id() == input.entity_id()
                    )
                })
                .cloned()
        }) else {
            return;
        };
        let value = input.read(cx).value().to_string();
        grid.update(cx, |table, cx| {
            table.delegate_mut().editing = None;
            cx.notify();
        });
        self.stage_cell_value(edit.row, edit.col, &value, cx);
    }

    pub(crate) fn set_local_cell(&mut self, row: usize, col: usize, value: &str, cx: &mut Context<Self>) {
        if let Some(cell) = self
            .rows
            .get_mut(row)
            .and_then(|cells| cells.get_mut(col))
        {
            *cell = value.to_string();
        }
        if let Some(grid) = self.grid.clone() {
            grid.update(cx, |table, cx| {
                if let Some(cell) = table
                    .delegate_mut()
                    .rows
                    .get_mut(row)
                    .and_then(|cells| cells.get_mut(col))
                {
                    *cell = value.to_string();
                }
                cx.notify();
            });
        }
        cx.notify();
    }

    pub(crate) fn sync_flags(&mut self, cx: &mut Context<Self>) {
        let Some(grid) = self.grid.clone() else {
            return;
        };
        let dirty: HashSet<(usize, usize)> = self.pending_edits.keys().copied().collect();
        let new_rows = self.new_rows.clone();
        let deleted_rows: HashSet<usize> = self.pending_deletes.keys().copied().collect();
        grid.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            delegate.dirty = dirty;
            delegate.new_rows = new_rows;
            delegate.deleted_rows = deleted_rows;
            cx.notify();
        });
    }

    pub(crate) fn has_pending_changes(&self) -> bool {
        !self.pending_edits.is_empty()
            || !self.new_rows.is_empty()
            || !self.pending_deletes.is_empty()
    }

    /// The table or query result that pending edits belong to.
    pub(crate) fn edit_target(&self) -> Option<EditTarget> {
        if self.show_query {
            let query = self
                .active_query
                .and_then(|index| self.queries.get(index))?;
            let target = self.query_edit.as_ref()?;
            if target.query_key != query.key {
                return None;
            }
            return Some(EditTarget {
                tab_key: query.key.clone(),
                page: 0,
                connection: target.connection.clone(),
                database: target.database.clone(),
                schema: target.schema.clone(),
                table: target.table.clone(),
                is_query: true,
            });
        }
        let tab = self.active.and_then(|index| self.tabs.get(index))?;
        let connection = self
            .connections
            .iter()
            .find(|item| item.id == tab.connection_id)?
            .clone();
        Some(EditTarget {
            tab_key: tab.key.clone(),
            page: self.table_page,
            connection,
            database: tab.database.clone(),
            schema: tab.schema.clone(),
            table: tab.table.clone(),
            is_query: false,
        })
    }

    pub(crate) fn add_row(&mut self, cx: &mut Context<Self>) {
        if self.columns.is_empty() || self.edit_target().is_none() {
            return;
        }
        let row = self.rows.len();
        let cells = vec![String::new(); self.columns.len()];
        self.rows.push(cells.clone());
        self.new_rows.insert(row);
        if let Some(grid) = self.grid.clone() {
            grid.update(cx, |table, cx| {
                table.delegate_mut().rows.push(cells);
                table.set_selected_cell(row, 1, cx);
                cx.notify();
            });
        }
        self.sync_flags(cx);
        cx.notify();
    }

    pub(crate) fn paste_row(
        &mut self,
        row: usize,
        values: &[String],
        cx: &mut Context<Self>,
    ) {
        if self.rows.get(row).is_none() || self.edit_target().is_none() {
            return;
        }
        let mut count = 0;
        for (col, value) in values.iter().enumerate() {
            if col >= self.columns.len() {
                break;
            }
            self.stage_cell_value(row, col, value, cx);
            count += 1;
        }
        if count > 0 {
            self.show_notice(&format!("已粘贴 {count} 列"), cx);
        }
    }

    pub(crate) fn delete_selected_row(&mut self, cx: &mut Context<Self>) {
        let Some(grid) = self.grid.clone() else {
            return;
        };
        let row = grid
            .read(cx)
            .selected_cell()
            .map(|(row, _)| row)
            .or_else(|| grid.read(cx).selected_row());
        match row {
            Some(row) => self.delete_row_at(row, cx),
            None => self.show_notice("请先选择一行", cx),
        }
    }

    pub(crate) fn delete_row_at(&mut self, row: usize, cx: &mut Context<Self>) {
        if self.rows.get(row).is_none() {
            return;
        }
        if self.new_rows.contains(&row) {
            self.rows.remove(row);
            self.new_rows.remove(&row);
            let shifted: HashSet<usize> = self
                .new_rows
                .drain()
                .map(|index| if index > row { index - 1 } else { index })
                .collect();
            self.new_rows = shifted;
            if let Some(grid) = self.grid.clone() {
                grid.update(cx, |table, cx| {
                    let delegate = table.delegate_mut();
                    if row < delegate.rows.len() {
                        delegate.rows.remove(row);
                    }
                    cx.notify();
                });
            }
            self.sync_flags(cx);
            cx.notify();
            return;
        }
        if self.pending_deletes.contains_key(&row) {
            return;
        }
        let Some(target) = self.edit_target() else {
            return;
        };
        let keys = match self.row_keys(row, &target.tab_key, target.page) {
            Ok(keys) => keys,
            Err(message) => {
                self.show_notice(&message, cx);
                return;
            }
        };
        self.pending_edits.retain(|(row_ix, _), _| *row_ix != row);
        self.pending_deletes.insert(row, keys);
        self.sync_flags(cx);
        cx.notify();
    }

    pub(crate) fn discard_pending_changes(&mut self, cx: &mut Context<Self>) -> bool {
        let had_changes = self.has_pending_changes();
        if let Some(grid) = self.grid.clone() {
            grid.update(cx, |table, cx| {
                table.delegate_mut().editing = None;
                cx.notify();
            });
        }
        let target = self.edit_target();
        let edits: Vec<((usize, usize), PendingEdit)> =
            self.pending_edits.drain().collect();
        for ((row, col), edit) in edits {
            if target
                .as_ref()
                .map_or(false, |target| target.tab_key == edit.tab_key && target.page == edit.page)
            {
                self.set_local_cell(row, col, &edit.original, cx);
            }
        }
        let new_count = self.new_rows.len();
        if new_count > 0 {
            let keep = self.rows.len().saturating_sub(new_count);
            self.rows.truncate(keep);
            if let Some(grid) = self.grid.clone() {
                grid.update(cx, |table, cx| {
                    table.delegate_mut().rows.truncate(keep);
                    cx.notify();
                });
            }
        }
        self.new_rows.clear();
        self.pending_deletes.clear();
        self.sync_flags(cx);
        cx.notify();
        had_changes
    }

    pub(crate) fn primary_key_values(
        &self,
        row: usize,
    ) -> Result<Vec<(String, String, String)>, String> {
        let primary_count = self
            .column_meta
            .iter()
            .filter(|meta| meta.is_primary_key)
            .count();
        let mut keys = Vec::with_capacity(primary_count);
        for meta in self.column_meta.iter().filter(|meta| meta.is_primary_key) {
            let Some(index) = self.columns.iter().position(|name| name == &meta.name) else {
                continue;
            };
            let Some(value) = self
                .rows
                .get(row)
                .and_then(|cells| cells.get(index))
                .cloned()
            else {
                continue;
            };
            keys.push((meta.name.clone(), meta.data_type.clone(), value));
        }
        if primary_count == 0 || keys.len() != primary_count {
            return Err("这张表没有主键，暂不支持编辑".to_string());
        }
        Ok(keys)
    }

    pub(crate) fn row_keys(
        &self,
        row: usize,
        tab_key: &str,
        page: usize,
    ) -> Result<Vec<(String, String, String)>, String> {
        if let Some((_, edit)) = self.pending_edits.iter().find(|((row_ix, _), edit)| {
            *row_ix == row && edit.tab_key == tab_key && edit.page == page
        }) {
            return Ok(edit.keys.clone());
        }
        self.primary_key_values(row)
    }

    pub(crate) fn stage_cell_value(
        &mut self,
        row: usize,
        col: usize,
        value: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.edit_target() else {
            return;
        };
        let tab_key = target.tab_key;
        let page = target.page;
        if self.pending_deletes.contains_key(&row) {
            return;
        }
        if self.new_rows.contains(&row) {
            self.set_local_cell(row, col, value, cx);
            cx.notify();
            return;
        }
        let Some(current) = self
            .rows
            .get(row)
            .and_then(|cells| cells.get(col))
            .cloned()
        else {
            return;
        };
        let Some(column) = self.columns.get(col).cloned() else {
            return;
        };
        let column_type = self
            .column_meta
            .get(col)
            .map(|meta| meta.data_type.clone())
            .unwrap_or_default();
        let original = self
            .pending_edits
            .get(&(row, col))
            .filter(|edit| edit.tab_key == tab_key && edit.page == page)
            .map(|edit| edit.original.clone())
            .unwrap_or(current);
        let keys = match self.row_keys(row, &tab_key, page) {
            Ok(keys) => keys,
            Err(message) => {
                self.show_notice(&message, cx);
                return;
            }
        };
        self.pending_edits.remove(&(row, col));
        let display = if value == original {
            original
        } else {
            self.pending_edits.insert(
                (row, col),
                PendingEdit {
                    tab_key: tab_key.clone(),
                    page,
                    column,
                    column_type,
                    original: original.clone(),
                    value: value.to_string(),
                    keys,
                },
            );
            value.to_string()
        };
        self.set_local_cell(row, col, &display, cx);
        self.sync_flags(cx);
        cx.notify();
    }

    pub(crate) fn cancel_pending_edits(&mut self, cx: &mut Context<Self>) {
        if self.discard_pending_changes(cx) {
            self.show_notice("已取消修改", cx);
        } else {
            self.show_notice("没有待取消的修改", cx);
        }
    }

    pub(crate) fn save_pending_edits(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        if let Some(grid) = self.grid.clone() {
            self.commit_editing(&grid, cx);
        }
        let Some(target) = self.edit_target() else {
            return;
        };
        self.pending_edits
            .retain(|_, edit| edit.tab_key == target.tab_key && edit.page == target.page);
        if !self.has_pending_changes() {
            self.show_notice("没有需要保存的修改", cx);
            return;
        }
        let connection = target.connection.clone();
        let mut statements = Vec::new();

        let mut delete_rows: Vec<usize> = self.pending_deletes.keys().copied().collect();
        delete_rows.sort_unstable();
        for row in delete_rows {
            let Some(keys) = self.pending_deletes.get(&row) else {
                continue;
            };
            match delete_row_sql(&connection, &target.table, keys) {
                Ok(sql) => statements.push(sql),
                Err(message) => {
                    self.show_notice(&message, cx);
                    return;
                }
            }
        }

        let mut grouped: HashMap<
            usize,
            (Vec<(String, String, String)>, Vec<(String, String, String)>),
        > = HashMap::new();
        for ((row, _), edit) in &self.pending_edits {
            let group = grouped
                .entry(*row)
                .or_insert_with(|| (Vec::new(), edit.keys.clone()));
            group.0.push((
                edit.column.clone(),
                edit.column_type.clone(),
                edit.value.clone(),
            ));
        }
        for (_, (assignments, keys)) in grouped {
            match update_row_sql(&connection, &target.table, &assignments, &keys) {
                Ok(sql) => statements.push(sql),
                Err(message) => {
                    self.show_notice(&message, cx);
                    return;
                }
            }
        }

        let mut new_rows: Vec<usize> = self.new_rows.iter().copied().collect();
        new_rows.sort_unstable();
        for row in new_rows {
            let Some(values) = self.rows.get(row) else {
                continue;
            };
            match insert_row_sql(
                &connection,
                &target.table,
                &self.columns,
                &self.column_meta,
                values,
            ) {
                Ok(sql) => statements.push(sql),
                Err(message) => {
                    self.show_notice(&message, cx);
                    return;
                }
            }
        }

        let count = self.pending_edits.len()
            + self.pending_deletes.len()
            + self.new_rows.len();
        let connection = connection.clone();
        let database = target.database.clone();
        let schema = target.schema.clone();
        let is_query = target.is_query;
        self.saving = true;
        let task = cx.background_executor().spawn(async move {
            for sql in statements {
                run_sql(&connection, &database, &schema, &sql, 1, None)?;
            }
            Ok::<(), String>(())
        });
        cx.spawn_in(window, async move |weak, cx| {
            let result = task.await;
            let _ = weak.update_in(cx, |this, window, cx| {
                this.saving = false;
                match result {
                    Ok(()) => {
                        this.pending_edits.clear();
                        this.pending_deletes.clear();
                        this.new_rows.clear();
                        this.sync_flags(cx);
                        this.show_notice(&format!("已保存 {count} 处修改"), cx);
                        if is_query {
                            this.run_query(window, cx);
                        } else {
                            this.load_active(window, cx);
                        }
                    }
                    Err(error) => this.show_notice(&format!("保存失败: {error}"), cx),
                }
            });
        })
        .detach();
    }

    pub(crate) fn filter_by_cell(
        &mut self,
        row: usize,
        col: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.show_query {
            return;
        }
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == tab.connection_id)
        else {
            return;
        };
        let Some(value) = self
            .rows
            .get(row)
            .and_then(|cells| cells.get(col))
            .cloned()
        else {
            return;
        };
        let Some(column) = self.columns.get(col).cloned() else {
            return;
        };
        let column_type = self
            .column_meta
            .get(col)
            .map(|meta| meta.data_type.clone())
            .unwrap_or_default();
        let clause = if value == "NULL" {
            format!("{} IS NULL", connection.quote(&column))
        } else {
            format!(
                "{} = {}",
                connection.quote(&column),
                edit_sql_literal(&value, &column_type, &connection.db_type)
            )
        };
        self.filters_visible = true;
        self.table_where
            .update(cx, |state, cx| state.set_value(clause.clone(), window, cx));
        self.where_value = clause;
        self.where_open = false;
        self.table_page = 1;
        self.load_active(window, cx);
    }

    pub(crate) fn sort_by_cell(
        &mut self,
        col: usize,
        ascending: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.show_query {
            return;
        }
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == tab.connection_id)
        else {
            return;
        };
        let Some(column) = self.columns.get(col).cloned() else {
            return;
        };
        let clause = format!(
            "{} {}",
            connection.quote(&column),
            if ascending { "ASC" } else { "DESC" }
        );
        self.filters_visible = true;
        self.table_order
            .update(cx, |state, cx| state.set_value(clause.clone(), window, cx));
        self.order_value = clause;
        self.order_open = false;
        self.table_page = 1;
        self.load_active(window, cx);
    }

    pub(crate) fn clear_where(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_query {
            return;
        }
        self.table_where
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.where_value.clear();
        self.where_open = false;
        self.table_page = 1;
        self.load_active(window, cx);
    }

    pub(crate) fn remove_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_query {
            return;
        }
        self.table_where
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.table_order
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.where_value.clear();
        self.order_value.clear();
        self.where_open = false;
        self.order_open = false;
        self.table_page = 1;
        self.table_has_more = false;
        self.load_active(window, cx);
    }

    pub(crate) fn check_for_update(&self, cx: &mut Context<Self>) {
        let task = cx.background_executor().spawn(async { fetch_update() });
        cx.spawn(async move |weak, cx| {
            let update = task.await;
            let _ = weak.update(cx, |this, cx| {
                this.update = update;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        match fetch_connections() {
            Ok(list) => {
                self.connections = list;
                self.error = None;
                self.databases.clear();
                self.schemas.clear();
                self.tables.clear();
            }
            Err(message) => self.error = Some(message),
        }
        cx.notify();
    }

    /// Section header above the connection tree: title on the left, search and
    /// connection actions on the right. Clicking search swaps the whole row for
    /// an input, matching the reference layout.
    pub(crate) fn connections_header(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        if self.search_open {
            return h_flex()
                .mx_3()
                .mt_3()
                .mb_2()
                .h(px(32.))
                .px_2()
                .rounded_lg()
                .bg(colors.background)
                .border_1()
                .border_color(colors.border)
                .child(
                    Input::new(&self.search)
                        .appearance(false)
                        .bordered(false)
                        .text_size(px(13.))
                        .cleanable(true)
                        .prefix(
                            Icon::new(AssetIcon::Search)
                                .with_size(px(16.))
                                .text_color(muted),
                        ),
                )
                .into_any_element();
        }

        let weak = cx.entity().downgrade();
        let focus_handle = self.focus_handle.clone();
        let settings = Button::new("connections-settings")
            .ghost()
            .with_size(gpui_kit::component::Size::Small)
            .icon(
                Icon::new(AssetIcon::SlidersHorizontal)
                    .with_size(px(16.))
                    .text_color(muted),
            )
            .dropdown_menu_with_anchor(gpui_kit::Anchor::TopRight, move |menu, _, cx| {
                let menu = app_menu(menu, cx)
                    .action_context(focus_handle.clone())
                    .min_w(px(180.));
                let new_connection = weak.clone();
                let close_all = weak.clone();
                let manage = weak.clone();
                let new_group = weak.clone();
                menu.item(PopupMenuItem::new("新建连接").on_click(move |_, _, cx| {
                    let _ = new_connection.update(cx, |this, cx| {
                        this.show_notice("连接向导未完成", cx)
                    });
                }))
                .item(
                    PopupMenuItem::new("关闭所有连接").on_click(move |_, _, cx| {
                        let _ = close_all.update(cx, |this, cx| {
                            this.open_connections.clear();
                            this.databases.clear();
                            this.tables.clear();
                            cx.notify();
                        });
                    }),
                )
                .item(PopupMenuItem::new("管理连接...").on_click(move |_, _, cx| {
                    let _ = manage.update(cx, |this, cx| this.show_notice("管理连接未完成", cx));
                }))
                .separator()
                .item(PopupMenuItem::new("新建组").on_click(move |_, _, cx| {
                    let _ = new_group.update(cx, |this, cx| this.show_notice("分组未完成", cx));
                }))
                .separator()
                .item(
                    PopupMenuItem::new("刷新")
                        .action(Box::new(RefreshConnections))
                        .on_click({
                            let refresh = weak.clone();
                            move |_, _, cx| {
                                let _ = refresh.update(cx, |this, cx| this.reload(cx));
                            }
                        }),
                )
            });

        h_flex()
            .mx_2()
            .mt_3()
            .mb_2()
            .h(px(32.))
            .pl_3()
            .pr_1()
            .gap_1()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child("我的链接"),
            )
            .child(ui::button::icon_button(
                "connections-search",
                Icon::new(AssetIcon::Search)
                    .with_size(px(16.))
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| {
                    this.search_open = true;
                    this.search
                        .update(cx, |state, cx| state.focus(window, cx));
                    cx.notify();
                }),
            ))
            .child(settings)
            .child(ui::button::icon_button(
                "connections-locate",
                Icon::new(AssetIcon::LocateFixed)
                    .with_size(px(16.))
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| this.locate_active_table(window, cx)),
            ))
            .into_any_element()
    }

    pub(crate) fn show_notice(&mut self, message: &str, cx: &mut Context<Self>) {
        self.notice = Some(message.to_string());
        self.notice_id = self.notice_id.wrapping_add(1);
        let id = self.notice_id;
        cx.notify();
        cx.spawn(async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(2))
                .await;
            let _ = weak.update(cx, |this, cx| {
                if this.notice_id == id {
                    this.notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn open_width_dialog(
        &mut self,
        target: WidthTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(grid) = self.grid.clone() else {
            return;
        };
        let value = {
            let table = grid.read(cx);
            let delegate = table.delegate();
            match target {
                WidthTarget::Column(data) => delegate.effective_width(data),
                WidthTarget::All => (0..delegate.columns.len())
                    .find(|data| !delegate.hidden.contains(data))
                    .map(|data| delegate.effective_width(data))
                    .unwrap_or(120.),
                WidthTarget::RowHeight => self.grid_row_height * self.grid_zoom,
            }
        };
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(format!("{:.0}", value), window, cx);
            state
        });
        input.update(cx, |state, cx| state.focus(window, cx));
        self.width_dialog = Some(WidthDialog {
            target,
            input,
            error: None,
        });
        cx.notify();
    }

    pub(crate) fn apply_width_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.width_dialog.take() else {
            return;
        };
        let raw = dialog.input.read(cx).value().trim().to_string();
        let min = match dialog.target {
            WidthTarget::RowHeight => 16.,
            _ => 40.,
        };
        let parsed = raw.parse::<f32>().ok().filter(|value| *value >= min);
        let Some(value) = parsed else {
            self.width_dialog = Some(WidthDialog {
                target: dialog.target,
                input: dialog.input,
                error: Some(format!("请输入不小于 {min} 的数字")),
            });
            cx.notify();
            return;
        };
        let zoom = self.grid_zoom.max(0.1);
        let base = value / zoom;
        if matches!(dialog.target, WidthTarget::RowHeight) {
            self.grid_row_height = base.clamp(16., 120.);
            cx.notify();
            return;
        }
        let Some(grid) = self.grid.clone() else {
            return;
        };
        grid.update(cx, |table, cx| {
            {
                let delegate = table.delegate();
                match dialog.target {
                    WidthTarget::Column(data) => {
                        delegate.widths.borrow_mut().insert(data, base);
                    }
                    WidthTarget::All => {
                        let visible: Vec<usize> = (0..delegate.columns.len())
                            .filter(|data| !delegate.hidden.contains(data))
                            .collect();
                        let mut widths = delegate.widths.borrow_mut();
                        for data in visible {
                            widths.insert(data, base);
                        }
                    }
                    WidthTarget::RowHeight => {}
                }
            }
            table.refresh(cx);
            cx.notify();
        });
        cx.notify();
    }

    pub(crate) fn toggle_column_hidden(
        &mut self,
        data_col: usize,
        hidden: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(grid) = self.grid.clone() else {
            return;
        };
        let mut delegate = grid.read(cx).delegate().clone();
        if hidden {
            delegate.hidden.insert(data_col);
        } else {
            delegate.hidden.remove(&data_col);
        }
        if delegate.frozen >= delegate.visible_len() {
            delegate.frozen = 0;
        }
        self.install_grid(delegate, window, cx);
        cx.notify();
    }

    pub(crate) fn export_table(
        &mut self,
        format: ExportFormat,
        all_pages: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            self.show_notice("没有可导出的表", cx);
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == tab.connection_id)
            .cloned()
        else {
            self.show_notice("连接不可用", cx);
            return;
        };
        let columns = self.columns.clone();
        let rows = self.rows.clone();
        let column_types = self
            .column_meta
            .iter()
            .map(|meta| meta.data_type.clone())
            .collect::<Vec<_>>();
        let where_clause = self.table_where.read(cx).value().trim().to_string();
        let order_clause = self.table_order.read(cx).value().trim().to_string();
        let page = self.table_page;
        let suffix = if all_pages {
            String::new()
        } else {
            format!("_page{page}")
        };
        let file_name = format!(
            "{}{}.{}",
            safe_export_stem(&tab.table),
            suffix,
            format.extension()
        );
        let directory = export_directory();
        let receiver = cx.prompt_for_new_path(&directory, Some(&file_name));
        cx.spawn(async move |weak, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = weak.update(cx, |this, cx| {
                        this.show_notice(&format!("导出失败: {error}"), cx)
                    });
                    return;
                }
                Err(_) => return,
            };
            let table = tab.table.clone();
            let task = cx.background_executor().spawn(async move {
                let data = if all_pages {
                    fetch_all_rows(
                        &connection,
                        &tab.database,
                        &tab.schema,
                        &table,
                        &where_clause,
                        &order_clause,
                    )?
                } else {
                    TableData {
                        columns,
                        column_types,
                        rows,
                        has_more: false,
                    }
                };
                let count = data.rows.len();
                write_export(&path, format, &data, &table, &connection)?;
                Ok::<usize, String>(count)
            });
            let result = task.await;
            let _ = weak.update(cx, |this, cx| match result {
                Ok(count) => this.show_notice(&format!("已导出 {count} 行"), cx),
                Err(error) => this.show_notice(&format!("导出失败: {error}"), cx),
            });
        })
        .detach();
    }

    pub(crate) fn export_query(&mut self, format: ExportFormat, cx: &mut Context<Self>) {
        if self.columns.is_empty() {
            self.show_notice("没有可导出的查询结果", cx);
            return;
        }
        let Some((connection, _, _)) = self.query_target() else {
            self.show_notice("连接不可用", cx);
            return;
        };
        let table = self
            .query_edit
            .as_ref()
            .map(|target| target.table.clone())
            .or_else(|| {
                self.query_info
                    .as_ref()
                    .and_then(|info| sql_tables(&info.sql).into_iter().next())
            })
            .unwrap_or_else(|| "query_result".to_string());
        let file_name = format!("{}.{}", safe_export_stem(&table), format.extension());
        let data = TableData {
            columns: self.columns.clone(),
            column_types: self
                .column_meta
                .iter()
                .map(|meta| meta.data_type.clone())
                .collect(),
            rows: self.rows.clone(),
            has_more: false,
        };
        let directory = export_directory();
        let receiver = cx.prompt_for_new_path(&directory, Some(&file_name));
        cx.spawn(async move |weak, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = weak.update(cx, |this, cx| {
                        this.show_notice(&format!("导出失败: {error}"), cx)
                    });
                    return;
                }
                Err(_) => return,
            };
            let count = data.rows.len();
            let task = cx.background_executor().spawn(async move {
                write_export(&path, format, &data, &table, &connection)?;
                Ok::<usize, String>(count)
            });
            let result = task.await;
            let _ = weak.update(cx, |this, cx| match result {
                Ok(count) => this.show_notice(&format!("已导出 {count} 行"), cx),
                Err(error) => this.show_notice(&format!("导出失败: {error}"), cx),
            });
        })
        .detach();
    }

    pub(crate) fn open_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_running_query();
        self.hide_props_now(cx);
        self.close_query_menus();
        self.next_query += 1;
        let number = self.next_query;
        let (connection_id, database, schema) = self.new_query_target();
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .default_value(
                    "SELECT id, name\nFROM customers\nWHERE id > 10\nORDER BY id\nLIMIT 50",
                )
                .language("sql")
        });
        self.queries.push(QueryTab {
            key: format!("query-{number}"),
            title: format!("查询 {number}"),
            editor,
            connection_id,
            database,
            schema,
        });
        self.active_query = Some(self.queries.len() - 1);
        self.show_query = true;
        self.tab_scroll
            .scroll_to_item(self.tabs.len() + self.queries.len() - 1);
        cx.notify();
    }

    /// Connection the query runs against: the active query tab, else the
    /// active table tab, else the first connection.
    pub(crate) fn query_target(&self) -> Option<(Connection, String, String)> {
        if self.show_query {
            if let Some(query) = self.active_query.and_then(|index| self.queries.get(index)) {
                if !query.connection_id.is_empty() {
                    if let Some(connection) = self
                        .connections
                        .iter()
                        .find(|item| item.id == query.connection_id)
                    {
                        return Some((
                            connection.clone(),
                            query.database.clone(),
                            query.schema.clone(),
                        ));
                    }
                }
            }
        }
        let tab = self.active.and_then(|index| self.tabs.get(index));
        if let Some(tab) = tab {
            let connection = self
                .connections
                .iter()
                .find(|item| item.id == tab.connection_id)?
                .clone();
            return Some((connection, tab.database.clone(), tab.schema.clone()));
        }
        let connection = self.connections.first()?.clone();
        let database = self
            .databases
            .get(&connection.id)
            .and_then(|list| list.last())
            .cloned()
            .unwrap_or_default();
        let schema = connection
            .default_schema
            .clone()
            .unwrap_or_else(|| database.clone());
        Some((connection, database, schema))
    }

    pub(crate) fn ensure_databases(&mut self, connection_id: &str) -> Vec<String> {
        if connection_id.is_empty() {
            return Vec::new();
        }
        if !self.databases.contains_key(connection_id) {
            if let Ok(list) = fetch_databases(connection_id) {
                self.databases.insert(connection_id.to_string(), list);
            }
        }
        self.databases.get(connection_id).cloned().unwrap_or_default()
    }

    pub(crate) fn ensure_schemas(&mut self, connection_id: &str, database: &str) -> Vec<String> {
        if connection_id.is_empty() || database.is_empty() {
            return Vec::new();
        }
        let key = format!("{connection_id}|{database}");
        if !self.schemas.contains_key(&key) {
            if let Ok(list) = fetch_schemas(connection_id, database) {
                self.schemas.insert(key.clone(), list);
            }
        }
        self.schemas.get(&key).cloned().unwrap_or_default()
    }

    /// MySQL-style connections use the database name as the schema; others
    /// fall back to the connection default, then the first server schema.
    pub(crate) fn resolve_query_schema(&mut self, connection: &Connection, database: &str) -> String {
        if database.is_empty() {
            return String::new();
        }
        let schemas = self.ensure_schemas(&connection.id, database);
        if schemas.iter().any(|item| item == database) {
            return database.to_string();
        }
        connection
            .default_schema
            .clone()
            .filter(|schema| !schema.is_empty())
            .or_else(|| schemas.first().cloned())
            .unwrap_or_else(|| database.to_string())
    }

    pub(crate) fn new_query_target(&mut self) -> (String, String, String) {
        let Some((connection, mut database, mut schema)) = self.query_target() else {
            return (String::new(), String::new(), String::new());
        };
        if database.is_empty() {
            database = self
                .ensure_databases(&connection.id)
                .last()
                .cloned()
                .unwrap_or_default();
        } else {
            self.ensure_databases(&connection.id);
        }
        self.ensure_schemas(&connection.id, &database);
        if schema.is_empty() {
            schema = self.resolve_query_schema(&connection, &database);
        }
        (connection.id, database, schema)
    }

    pub(crate) fn format_query_sql(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(query) = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .cloned()
        else {
            return;
        };
        let sql = query.editor.read(cx).value().to_string();
        if sql.trim().is_empty() {
            return;
        }
        let db_type = self
            .connections
            .iter()
            .find(|item| item.id == query.connection_id)
            .map(|item| item.db_type.clone())
            .unwrap_or_default();
        let formatted = format_sql_text(&sql, &db_type);
        query
            .editor
            .update(cx, |state, cx| state.set_value(&formatted, window, cx));
        self.result_tab = ResultTab::Results;
        cx.notify();
    }

    pub(crate) fn close_query_menus(&mut self) {
        self.query_connection_open = false;
        self.query_database_open = false;
        self.query_schema_open = false;
    }

    pub(crate) fn set_query_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.active_query else {
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == connection_id)
            .cloned()
        else {
            return;
        };
        let database = self
            .ensure_databases(&connection.id)
            .last()
            .cloned()
            .unwrap_or_default();
        let schema = self.resolve_query_schema(&connection, &database);
        if let Some(query) = self.queries.get_mut(index) {
            query.connection_id = connection.id;
            query.database = database;
            query.schema = schema;
        }
        self.close_query_menus();
        cx.notify();
    }

    pub(crate) fn set_query_database(&mut self, database: &str, cx: &mut Context<Self>) {
        let Some(index) = self.active_query else {
            return;
        };
        let Some(query) = self.queries.get(index).cloned() else {
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == query.connection_id)
            .cloned()
        else {
            return;
        };
        let schema = self.resolve_query_schema(&connection, database);
        if let Some(query) = self.queries.get_mut(index) {
            query.database = database.to_string();
            query.schema = schema;
        }
        self.close_query_menus();
        cx.notify();
    }

    pub(crate) fn set_query_schema(&mut self, schema: &str, cx: &mut Context<Self>) {
        let Some(index) = self.active_query else {
            return;
        };
        if let Some(query) = self.queries.get_mut(index) {
            query.schema = schema.to_string();
        }
        self.close_query_menus();
        cx.notify();
    }

    pub(crate) fn refresh_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_query {
            self.run_query(window, cx);
        } else {
            self.load_active(window, cx);
        }
    }

    pub(crate) fn run_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.run_query_inner(None, true, window, cx);
    }

    pub(crate) fn explain_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(query) = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .cloned()
        else {
            return;
        };
        let sql = query.editor.read(cx).value().to_string();
        if sql.trim().is_empty() {
            return;
        }
        let Some((connection, _, _)) = self.query_target() else {
            self.error = Some("没有可用的连接".to_string());
            cx.notify();
            return;
        };
        self.run_query_inner(Some(explain_sql(&connection, &sql)), false, window, cx);
    }

    fn run_query_inner(
        &mut self,
        sql_override: Option<String>,
        allow_edit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(query) = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .cloned()
        else {
            return;
        };
        let query_key = query.key.clone();
        let sql = sql_override.unwrap_or_else(|| query.editor.read(cx).value().to_string());
        if sql.trim().is_empty() {
            return;
        }
        let Some((connection, database, schema)) = self.query_target() else {
            self.error = Some("没有可用的连接".to_string());
            cx.notify();
            return;
        };
        if let Some(previous) = self.running_query.take() {
            Self::cancel_query_id(previous);
        }
        self.loading = true;
        let execution_id = uuid::Uuid::new_v4().to_string();
        self.running_query = Some(execution_id.clone());
        self.cancel_requested = None;
        self.result_tab = ResultTab::Results;
        self.query_edit = None;
        self.pending_edits.clear();
        self.new_rows.clear();
        self.pending_deletes.clear();
        self.query_info = Some(QueryInfo {
            sql: sql.clone(),
            connection: connection.name.clone(),
            database: database.clone(),
            schema: schema.clone(),
            elapsed_ms: None,
            rows: 0,
            columns: 0,
            status: QueryStatus::Running,
            error: None,
        });
        cx.notify();
        let started = std::time::Instant::now();
        let task_id = execution_id.clone();
        let sql_for_tables = sql.clone();
        let query_connection = connection.clone();
        let query_database = database.clone();
        let query_schema = schema.clone();
        let task = cx.background_executor().spawn(async move {
            let data = run_sql(
                &connection,
                &database,
                &schema,
                &sql,
                ROW_LIMIT,
                Some(&task_id),
            )?;
            let tables = sql_tables(&sql_for_tables);
            let edit_table = if tables.len() == 1 {
                tables.first().cloned()
            } else {
                None
            };
            let mut schema_columns: Vec<ColumnMeta> = Vec::new();
            for table in tables {
                if let Ok(columns) =
                    fetch_columns(&connection.id, &database, &schema, &table)
                {
                    for column in columns {
                        if !schema_columns.iter().any(|item| item.name == column.name) {
                            schema_columns.push(column);
                        }
                    }
                }
            }
            Ok::<(TableData, Vec<ColumnMeta>, Option<String>), String>((
                data,
                schema_columns,
                edit_table,
            ))
        });
        cx.spawn_in(window, async move |weak, cx| {
            let result = task.await;
            let _ = weak.update_in(cx, |this, window, cx| {
                if this.running_query.as_deref() != Some(execution_id.as_str()) {
                    return;
                }
                let cancelled =
                    this.cancel_requested.as_deref() == Some(execution_id.as_str());
                this.running_query = None;
                this.cancel_requested = None;
                this.loading = false;
                match result {
                    Ok((data, schema_columns, edit_table)) => {
                        let rows_len = data.rows.len();
                        let columns_len = data.columns.len();
                        let elapsed_ms = started.elapsed().as_millis();
                        let unique_columns = {
                            let mut names = HashSet::new();
                            data.columns.iter().all(|name| names.insert(name.as_str()))
                        };
                        let all_mapped = unique_columns
                            && data.columns.iter().all(|name| {
                                schema_columns.iter().any(|column| &column.name == name)
                            });
                        let meta: Vec<ColumnMeta> = data
                            .columns
                            .iter()
                            .enumerate()
                            .map(|(index, name)| {
                                let mut column = schema_columns
                                    .iter()
                                    .find(|item| &item.name == name)
                                    .cloned()
                                    .unwrap_or_default();
                                column.name = name.clone();
                                if column.data_type.is_empty() {
                                    column.data_type = data
                                        .column_types
                                        .get(index)
                                        .cloned()
                                        .unwrap_or_default();
                                }
                                column
                            })
                            .collect();
                        let editable = allow_edit
                            && edit_table.is_some()
                            && all_mapped
                            && meta.iter().any(|column| column.is_primary_key);
                        this.query_edit = if editable {
                            edit_table.as_ref().map(|table| QueryEditTarget {
                                query_key: query_key.clone(),
                                connection: query_connection.clone(),
                                database: query_database.clone(),
                                schema: query_schema.clone(),
                                table: table.clone(),
                            })
                        } else {
                            None
                        };
                        let grid_connection =
                            this.query_edit.as_ref().map(|target| target.connection.clone());
                        let grid_table =
                            this.query_edit.as_ref().map(|target| target.table.clone());
                        this.column_meta = meta.clone();
                        let delegate = GridDelegate {
                            columns: data.columns.clone(),
                            rows: data.rows.clone(),
                            meta,
                            show_comments: this.show_comments,
                            show_types: this.grid_show_types,
                            zoom: this.grid_zoom,
                            hidden: HashSet::new(),
                            widths: Rc::new(RefCell::new(HashMap::new())),
                            viewport: this.grid_viewport.clone(),
                            frozen: 0,
                            app: cx.entity().downgrade(),
                            connection: grid_connection,
                            table_name: grid_table,
                            row_offset: 0,
                            editing: None,
                            dirty: HashSet::new(),
                            new_rows: HashSet::new(),
                            deleted_rows: HashSet::new(),
                            weak: Some(cx.entity().downgrade()),
                            editable,
                        };
                        this.install_grid(delegate, window, cx);
                        this.columns = data.columns;
                        this.rows = data.rows;
                        this.error = None;
                        this.query_ms = Some(elapsed_ms);
                        if let Some(info) = this.query_info.as_mut() {
                            info.elapsed_ms = Some(elapsed_ms);
                            info.rows = rows_len;
                            info.columns = columns_len;
                            info.status = QueryStatus::Success;
                            info.error = None;
                        }
                    }
                    Err(message) => {
                        this.query_edit = None;
                        let elapsed_ms = started.elapsed().as_millis();
                        if let Some(info) = this.query_info.as_mut() {
                            info.elapsed_ms = Some(elapsed_ms);
                            info.status = if cancelled {
                                QueryStatus::Cancelled
                            } else {
                                QueryStatus::Failed
                            };
                            info.error = if cancelled {
                                None
                            } else {
                                Some(message.clone())
                            };
                        }
                        if cancelled {
                            this.show_notice("已停止查询", cx);
                        } else {
                            this.grid = None;
                            this.columns = Vec::new();
                            this.rows = Vec::new();
                            this.error = Some(message);
                        }
                        this.query_ms = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn close_query(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.queries.len() {
            return;
        }
        self.close_query_menus();
        let was_active = self.show_query && self.active_query == Some(index);
        self.queries.remove(index);
        self.active_query = match self.active_query {
            Some(active) if index < active => Some(active - 1),
            Some(active) if index == active => None,
            other => other,
        };
        if self.queries.is_empty() {
            self.active_query = None;
            self.show_query = false;
            if self.active.is_some() {
                self.load_active(window, cx);
            } else {
                self.grid = None;
                self.columns.clear();
                self.rows.clear();
                cx.notify();
            }
        } else if was_active {
            let next = index.min(self.queries.len() - 1);
            self.active_query = Some(next);
            self.show_query = true;
            cx.notify();
        } else {
            cx.notify();
        }
    }

    pub(crate) fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.cancel_running_query();
            self.hide_props_now(cx);
            self.active = None;
            self.grid = None;
            self.columns.clear();
            self.rows.clear();
        } else {
            let next = index.min(self.tabs.len() - 1);
            self.active = Some(next);
            self.load_active(window, cx);
            self.sync_props_if_open(window, cx);
            return;
        }
        cx.notify();
    }

    pub(crate) fn close_others(&mut self, target: CloseTarget, window: &mut Window, cx: &mut Context<Self>) {
        match target {
            CloseTarget::Table(index) => {
                if index >= self.tabs.len() {
                    return;
                }
                let tab = self.tabs[index].clone();
                let was_active = !self.show_query && self.active == Some(index);
                self.tabs = vec![tab];
                self.queries.clear();
                self.active = Some(0);
                self.active_query = None;
                self.show_query = false;
                if !was_active {
                    self.clear_table_filters(window, cx);
                }
                self.load_active(window, cx);
            }
            CloseTarget::Query(index) => {
                if index >= self.queries.len() {
                    return;
                }
                let query = self.queries[index].clone();
                self.queries = vec![query];
                self.tabs.clear();
                self.active = None;
                self.active_query = Some(0);
                self.show_query = true;
                self.grid = None;
                self.columns.clear();
                self.rows.clear();
                self.error = None;
                cx.notify();
            }
        }
    }

    pub(crate) fn close_right(&mut self, target: CloseTarget, window: &mut Window, cx: &mut Context<Self>) {
        match target {
            CloseTarget::Table(index) => {
                if index >= self.tabs.len() {
                    return;
                }
                let active_table = if self.show_query { None } else { self.active };
                self.tabs.truncate(index + 1);
                self.queries.clear();
                self.active_query = None;
                if self.show_query || active_table.map_or(true, |active| active > index) {
                    self.show_query = false;
                    self.active = Some(index);
                    self.load_active(window, cx);
                } else {
                    cx.notify();
                }
            }
            CloseTarget::Query(index) => {
                if index >= self.queries.len() {
                    return;
                }
                self.queries.truncate(index + 1);
                if self.active_query.map_or(false, |active| active > index) {
                    self.active_query = Some(index);
                }
                if self.show_query && self.active_query.is_none() {
                    self.active_query = Some(index);
                }
                cx.notify();
            }
        }
    }

    pub(crate) fn close_all_tabs(&mut self, cx: &mut Context<Self>) {
        self.cancel_running_query();
        self.hide_props_now(cx);
        self.close_query_menus();
        self.tabs.clear();
        self.queries.clear();
        self.active = None;
        self.active_query = None;
        self.show_query = false;
        self.grid = None;
        self.columns.clear();
        self.rows.clear();
        self.error = None;
        cx.notify();
    }

    pub(crate) fn reveal_in_tree(&mut self, target: CloseTarget, window: &mut Window, cx: &mut Context<Self>) {
        let table = match target {
            CloseTarget::Table(index) => self.tabs.get(index).map(|tab| tab.table.clone()),
            CloseTarget::Query(_) => None,
        };
        let Some(table) = table else {
            self.show_notice("查询标签不在导航窗格中", cx);
            return;
        };
        self.search_open = true;
        self.filter = table.clone();
        self.search
            .update(cx, |state, cx| state.set_value(table, window, cx));
        cx.notify();
    }
}

macro_rules! db_icon {
    ($name:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/db-icons/", $name, ".svg")) as &[u8]
    };
}
