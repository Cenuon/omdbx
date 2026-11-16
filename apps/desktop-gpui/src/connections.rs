use crate::*;

pub(crate) fn load_sidebar_layout_root() -> serde_json::Value {
    get_json(&format!("{API}/api/layout/sidebar"))
        .ok()
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| json!({}))
}

/// Keep one baseline snapshot per connection so the profile submenu has a
/// useful entry even before the user saves a named variant.
pub(crate) fn ensure_default_connection_profiles(
    connections: &[Connection],
    profiles: &mut HashMap<String, Vec<ConnectionProfile>>,
) -> bool {
    let mut changed = false;
    for connection in connections {
        if !profiles.contains_key(&connection.id) {
            profiles.insert(
                connection.id.clone(),
                vec![ConnectionProfile {
                    id: "default".to_string(),
                    name: "默认配置".to_string(),
                    config: sanitized_connection_config(connection.full_config()),
                }],
            );
            changed = true;
        }
    }
    changed
}

pub(crate) fn sanitized_connection_config(mut config: serde_json::Value) -> serde_json::Value {
    if let Some(object) = config.as_object_mut() {
        object.retain(|key, _| {
            let key = key.to_ascii_lowercase();
            !key.contains("password")
                && !key.contains("secret")
                && !key.contains("token")
                && key != "connection_string"
                && key != "transport_layers"
                && key != "init_script"
                && key != "database_info"
        });
    }
    config
}

impl Omdbx {
    pub(crate) fn persist_sidebar_layout(&self) {
        let mut root = load_sidebar_layout_root();
        if !root.is_object() {
            root = self.sidebar_layout_root.clone();
        }
        if !root.is_object() {
            root = json!({});
        }
        let settings = OmdbxSidebarSettings {
            folders: self.folders.clone(),
            stars: self.stars.clone(),
            profiles: self.connection_profiles.clone(),
            active_profiles: self.active_profiles.clone(),
            show_empty_databases: self.show_empty_databases,
        };
        root["omdbx"] = serde_json::to_value(settings).unwrap_or_else(|_| json!({}));
        let _ = post_json(
            &format!("{API}/api/layout/sidebar"),
            &json!({ "layout": root }),
        );
    }

    pub(crate) fn open_connection(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.databases.contains_key(id) {
            match fetch_databases(id) {
                Ok(list) => {
                    self.databases.insert(id.to_string(), list);
                    self.error = None;
                }
                Err(message) => {
                    self.error = Some(message.clone());
                    self.show_notice(&format!("打开连接失败: {message}"), cx);
                    return;
                }
            }
        }
        self.open_connections.insert(id.to_string());
        if let Some(name) = self
            .connections
            .iter()
            .find(|connection| connection.id == id)
            .map(|connection| connection.name.clone())
        {
            self.show_notice(&format!("已打开 {name}"), cx);
        }
        self.refresh_empty_database_status(cx);
        cx.notify();
    }

    pub(crate) fn toggle_star(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self.stars.iter().position(|id| id == connection_id) {
            self.stars.remove(index);
        } else {
            self.stars.push(connection_id.to_string());
        }
        self.persist_sidebar_layout();
        let starred = self.stars.iter().any(|id| id == connection_id);
        self.show_notice(if starred { "已添加星标" } else { "已取消星标" }, cx);
        cx.notify();
    }

    pub(crate) fn set_connection_color(
        &mut self,
        connection_id: &str,
        color: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            return;
        };
        let mut config = connection.full_config();
        match color {
            Some(color) if !color.is_empty() => config["color"] = json!(color),
            _ => {
                if let Some(object) = config.as_object_mut() {
                    object.remove("color");
                }
            }
        }
        match save_connection_config(&config) {
            Ok(()) => {
                self.active_profiles.remove(connection_id);
                self.reload(cx);
                self.show_notice("连接颜色已更新", cx);
            }
            Err(message) => self.show_notice(&format!("设置颜色失败: {message}"), cx),
        }
    }

    pub(crate) fn add_folder(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let folder = SidebarFolder {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            connections: Vec::new(),
        };
        self.open_folders.insert(folder.id.clone());
        self.folders.push(folder);
        self.persist_sidebar_layout();
        self.show_notice(&format!("已新建组「{name}」"), cx);
        cx.notify();
    }

    pub(crate) fn rename_folder(&mut self, folder_id: &str, name: &str, cx: &mut Context<Self>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(folder) = self
            .folders
            .iter_mut()
            .find(|folder| folder.id == folder_id)
        {
            folder.name = name.to_string();
            self.persist_sidebar_layout();
            self.show_notice(&format!("组已重命名为「{name}」"), cx);
            cx.notify();
        }
    }

    pub(crate) fn delete_folder(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        self.folders.retain(|folder| folder.id != folder_id);
        self.open_folders.remove(folder_id);
        self.persist_sidebar_layout();
        self.show_notice("已删除组，连接仍保留在列表中", cx);
        cx.notify();
    }

    pub(crate) fn move_connection_to_folder(
        &mut self,
        connection_id: &str,
        folder_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        for folder in &mut self.folders {
            folder.connections.retain(|id| id != connection_id);
        }
        if let Some(folder_id) = folder_id {
            if let Some(folder) = self.folders.iter_mut().find(|folder| folder.id == folder_id) {
                folder.connections.push(connection_id.to_string());
                self.open_folders.insert(folder_id.to_string());
            }
        }
        self.persist_sidebar_layout();
        self.show_notice(
            if folder_id.is_some() {
                "已移动到组"
            } else {
                "已移出组"
            },
            cx,
        );
        cx.notify();
    }

    pub(crate) fn open_new_folder_prompt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_text_prompt(
            "新建组",
            "输入组名，用来整理侧边栏连接",
            "组名",
            PromptAction::NewFolder,
            window,
            cx,
        );
    }

    pub(crate) fn confirm_delete_connection(
        &mut self,
        connection_id: &str,
        cx: &mut Context<Self>,
    ) {
        let name = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .map(|connection| connection.name.clone())
            .unwrap_or_else(|| "连接".to_string());
        self.confirm_dialog = Some(ConfirmDialog {
            title: "删除连接".to_string(),
            message: format!(
                "确定删除连接「{name}」吗？保存的密码、颜色、分组和配置文件也会一并移除。"
            ),
            confirm_label: "删除".to_string(),
            action: ConfirmAction::DeleteConnection(connection_id.to_string()),
        });
        cx.notify();
    }

    pub(crate) fn confirm_delete_folder(
        &mut self,
        folder_id: &str,
        cx: &mut Context<Self>,
    ) {
        let name = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .map(|folder| folder.name.clone())
            .unwrap_or_else(|| "组".to_string());
        self.confirm_dialog = Some(ConfirmDialog {
            title: "删除组".to_string(),
            message: format!("确定删除组「{name}」吗？组内连接不会被删除。"),
            confirm_label: "删除组".to_string(),
            action: ConfirmAction::DeleteFolder(folder_id.to_string()),
        });
        cx.notify();
    }

    pub(crate) fn submit_confirm_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.confirm_dialog.take() else {
            return;
        };
        match dialog.action {
            ConfirmAction::DeleteConnection(id) => self.delete_connection(&id, cx),
            ConfirmAction::DeleteFolder(id) => self.delete_folder(&id, cx),
        }
    }

    pub(crate) fn open_rename_folder_prompt(
        &mut self,
        folder_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(folder) = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .cloned()
        else {
            return;
        };
        self.open_text_prompt(
            "重命名组",
            "修改侧边栏中的组名",
            "组名",
            PromptAction::RenameFolder(folder.id),
            window,
            cx,
        );
        if let Some(prompt) = self.text_prompt.as_mut() {
            prompt.input.update(cx, |input, cx| {
                input.set_value(folder.name, window, cx);
            });
        }
    }

    pub(crate) fn open_new_profile_prompt(
        &mut self,
        connection_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_text_prompt(
            "保存连接配置文件",
            "保存当前主机、端口和数据库等非敏感设置，后续可一键切换",
            "配置文件名称",
            PromptAction::NewProfile(connection_id.to_string()),
            window,
            cx,
        );
    }

    fn open_text_prompt(
        &mut self,
        title: &str,
        subtitle: &str,
        placeholder: &str,
        action: PromptAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        input.update(cx, |input, cx| input.focus(window, cx));
        self.text_prompt = Some(TextPromptDialog {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            input,
            action,
            error: None,
        });
        cx.notify();
    }

    pub(crate) fn submit_text_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(prompt) = self.text_prompt.as_ref() else {
            return;
        };
        let value = prompt.input.read(cx).value().trim().to_string();
        if value.is_empty() {
            if let Some(prompt) = self.text_prompt.as_mut() {
                prompt.error = Some("名称不能为空".to_string());
            }
            cx.notify();
            return;
        }
        let action = match &prompt.action {
            PromptAction::NewFolder => PromptAction::NewFolder,
            PromptAction::RenameFolder(id) => PromptAction::RenameFolder(id.clone()),
            PromptAction::NewProfile(id) => PromptAction::NewProfile(id.clone()),
        };
        match action {
            PromptAction::NewFolder => self.add_folder(&value, cx),
            PromptAction::RenameFolder(id) => self.rename_folder(&id, &value, cx),
            PromptAction::NewProfile(id) => self.save_connection_profile(&id, &value, cx),
        }
        self.text_prompt = None;
        cx.notify();
    }

    pub(crate) fn save_connection_profile(
        &mut self,
        connection_id: &str,
        name: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            return;
        };
        self.connection_profiles
            .entry(connection_id.to_string())
            .or_default()
            .push(ConnectionProfile {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                config: sanitized_connection_config(connection.full_config()),
            });
        self.persist_sidebar_layout();
        self.show_notice(&format!("已保存配置文件「{name}」"), cx);
        cx.notify();
    }

    pub(crate) fn switch_connection_profile(
        &mut self,
        connection_id: &str,
        profile_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            return;
        };
        let Some(profile) = self
            .connection_profiles
            .get(connection_id)
            .and_then(|profiles| profiles.iter().find(|profile| profile.id == profile_id))
            .cloned()
        else {
            return;
        };
        let mut config = connection.full_config();
        if let Some(object) = profile.config.as_object() {
            for (key, value) in object {
                if key == "id" || key == "name" {
                    continue;
                }
                config[key] = value.clone();
            }
        }
        config["id"] = json!(connection.id);
        config["name"] = json!(connection.name);
        match save_connection_config(&config) {
            Ok(()) => {
                self.active_profiles
                    .insert(connection_id.to_string(), profile_id.to_string());
                self.persist_sidebar_layout();
                self.reload(cx);
                self.show_notice(&format!("已切换到「{}」", profile.name), cx);
            }
            Err(message) => self.show_notice(&format!("切换配置失败: {message}"), cx),
        }
    }

    pub(crate) fn delete_active_profile(
        &mut self,
        connection_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(profile_id) = self.active_profiles.remove(connection_id) else {
            return;
        };
        if let Some(profiles) = self.connection_profiles.get_mut(connection_id) {
            profiles.retain(|profile| profile.id != profile_id);
        }
        self.persist_sidebar_layout();
        self.show_notice("已删除当前配置文件", cx);
        cx.notify();
    }

    pub(crate) fn share_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            return;
        };
        let config = sanitized_connection_config(connection.full_config());
        let text = serde_json::to_string_pretty(&config).unwrap_or_default();
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
        self.show_notice("已复制脱敏连接配置到剪贴板", cx);
    }

    pub(crate) fn duplicate_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            return;
        };
        let mut copy_name = format!("{} 副本", connection.name);
        let mut suffix = 2;
        while self
            .connections
            .iter()
            .any(|item| item.name == copy_name)
        {
            copy_name = format!("{} 副本 {}", connection.name, suffix);
            suffix += 1;
        }
        let copy_id = uuid::Uuid::new_v4().to_string();
        let result = post_json(
            &format!("{API}/api/connection/mcp/duplicate"),
            &json!({
                "sourceId": connection.id,
                "copyId": copy_id,
                "copyName": copy_name,
            }),
        );
        match result {
            Ok(_) => {
                let folder_id = self
                    .folders
                    .iter()
                    .find(|folder| folder.connections.contains(&connection.id))
                    .map(|folder| folder.id.clone());
                if let Some(folder_id) = folder_id {
                    if let Some(folder) = self.folders.iter_mut().find(|folder| folder.id == folder_id)
                    {
                        folder.connections.push(copy_id.clone());
                    }
                }
                if self.stars.contains(&connection.id) {
                    self.stars.push(copy_id.clone());
                }
                if let Some(profiles) = self.connection_profiles.get(&connection.id).cloned() {
                    self.connection_profiles.insert(copy_id.clone(), profiles);
                }
                if let Some(active) = self.active_profiles.get(&connection.id).cloned() {
                    self.active_profiles.insert(copy_id.clone(), active);
                }
                self.persist_sidebar_layout();
                self.reload(cx);
                self.show_notice(&format!("已复制为「{copy_name}」"), cx);
            }
            Err(message) => self.show_notice(&format!("复制连接失败: {message}"), cx),
        }
    }

    pub(crate) fn delete_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let result = post_json(
            &format!("{API}/api/connection/save"),
            &json!({ "configs": [], "removedIds": [connection_id] }),
        );
        if let Err(message) = result {
            self.show_notice(&format!("删除连接失败: {message}"), cx);
            return;
        }

        let removed_name = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .map(|connection| connection.name.clone())
            .unwrap_or_else(|| "连接".to_string());
        self.connections.retain(|connection| connection.id != connection_id);
        self.open_connections.remove(connection_id);
        self.databases.remove(connection_id);
        let prefix = format!("{connection_id}|");
        self.open_databases.retain(|key| !key.starts_with(&prefix));
        self.tables.retain(|key, _| !key.starts_with(&prefix));
        self.schemas.retain(|key, _| !key.starts_with(&prefix));
        self.database_has_tables.retain(|key, _| !key.starts_with(&prefix));
        for folder in &mut self.folders {
            folder.connections.retain(|id| id != connection_id);
        }
        self.stars.retain(|id| id != connection_id);
        self.connection_profiles.remove(connection_id);
        self.active_profiles.remove(connection_id);

        let active_tab_key = self
            .active
            .and_then(|index| self.tabs.get(index))
            .map(|tab| tab.key.clone());
        let active_query_key = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .map(|query| query.key.clone());
        let active_table_removed = active_tab_key
            .as_ref()
            .is_some_and(|key| !self.tabs.iter().any(|tab| &tab.key == key));
        let active_query_removed = active_query_key
            .as_ref()
            .is_some_and(|key| !self.queries.iter().any(|query| &query.key == key));
        self.tabs.retain(|tab| tab.connection_id != connection_id);
        self.queries.retain(|query| query.connection_id != connection_id);
        self.active = active_tab_key
            .and_then(|key| self.tabs.iter().position(|tab| tab.key == key));
        self.active_query = active_query_key
            .and_then(|key| self.queries.iter().position(|query| query.key == key));
        if self.active.is_none() && !self.tabs.is_empty() {
            self.active = Some(0);
        }
        if self.active_query.is_none() && !self.queries.is_empty() {
            self.active_query = Some(0);
        }
        if self.queries.is_empty() {
            self.show_query = false;
        }
        if active_table_removed || active_query_removed {
            self.grid = None;
            self.columns.clear();
            self.rows.clear();
        }

        self.persist_sidebar_layout();
        self.reload(cx);
        self.show_notice(&format!("已删除连接「{removed_name}」"), cx);
    }

    pub(crate) fn toggle_show_empty_databases(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_empty_databases = show;
        self.persist_sidebar_layout();
        if !show {
            self.refresh_empty_database_status(cx);
            self.show_notice("正在检测空数据库", cx);
        } else {
            self.show_notice("已显示空数据库", cx);
        }
        cx.notify();
    }

    pub(crate) fn refresh_empty_database_status(&mut self, cx: &mut Context<Self>) {
        if self.show_empty_databases {
            return;
        }
        let mut targets = Vec::new();
        for connection_id in &self.open_connections {
            let Some(connection) = self
                .connections
                .iter()
                .find(|connection| &connection.id == connection_id)
                .cloned()
            else {
                continue;
            };
            let Some(databases) = self.databases.get(connection_id).cloned() else {
                continue;
            };
            targets.push((connection, databases));
        }
        if targets.is_empty() {
            return;
        }
        let task = cx.background_executor().spawn(async move {
            let mut statuses = HashMap::new();
            for (connection, databases) in targets {
                for database in databases {
                    let key = format!("{}|{}", connection.id, database);
                    let schema = connection
                        .default_schema
                        .clone()
                        .or_else(|| {
                            fetch_schemas(&connection.id, &database)
                                .ok()
                                .and_then(|schemas| schemas.into_iter().next())
                        })
                        .unwrap_or_else(|| database.clone());
                    let has_tables = fetch_tables(&connection.id, &database, &schema)
                        .map(|tables| !tables.is_empty())
                        .unwrap_or(true);
                    statuses.insert(key, has_tables);
                }
            }
            statuses
        });
        cx.spawn(async move |weak, cx| {
            let statuses = task.await;
            let _ = weak.update(cx, |this, cx| {
                this.database_has_tables.extend(statuses);
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn refresh_connection(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let name = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .map(|connection| connection.name.clone())
            .unwrap_or_else(|| "连接".to_string());
        self.reload(cx);
        self.refresh_empty_database_status(cx);
        self.show_notice(&format!("已刷新 {name}"), cx);
    }
}
