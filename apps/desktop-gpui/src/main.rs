//! OMDBX desktop, Rust + GPUI, styled with GPUI Kit.
//! Talks to the existing `dbx-web` sidecar over HTTP so all connection and query
//! logic is shared with the Electron build.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable, Theme, ThemeMode,
};
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::prelude::*;
use gpui_kit::{
    div, px, App, Entity, InteractiveElement, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, WindowOptions,
};
use serde::Deserialize;
use serde_json::json;

const API: &str = "http://127.0.0.1:4224";
const ROW_LIMIT: usize = 200;

// ---------------------------------------------------------------- http

fn agent() -> ureq::Agent {
    // localhost must never go through the user's HTTP proxy, and a hung sidecar
    // should not freeze the UI thread.
    ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .timeout_connect(Duration::from_secs(2))
        .timeout_read(Duration::from_secs(10))
        .build()
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let body = agent()
        .get(url)
        .call()
        .map_err(|error| error.to_string())?
        .into_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

fn post_json(url: &str, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
    let body = agent()
        .post(url)
        .set("Content-Type", "application/json")
        .send_string(&payload.to_string())
        .map_err(|error| error.to_string())?
        .into_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct Connection {
    id: String,
    name: String,
    db_type: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    default_schema: Option<String>,
}

impl Connection {
    fn endpoint(&self) -> String {
        match (self.host.as_deref(), self.port) {
            (Some(host), Some(port)) if port > 0 => format!("{host}:{port}"),
            (Some(host), _) => host.to_string(),
            _ => "—".to_string(),
        }
    }

    /// Dialect identifier quoting (the server has an endpoint for this, but the
    /// convention below matches it for every backend we support).
    fn quote(&self, name: &str) -> String {
        match self.db_type.as_str() {
            "mysql" | "goldendb" | "databend" => format!("`{}`", name.replace('`', "``")),
            "sqlserver" => format!("[{}]", name.replace(']', "]]")),
            _ => format!("\"{}\"", name.replace('"', "\"\"")),
        }
    }
}

fn fetch_connections() -> Result<Vec<Connection>, String> {
    let value = get_json(&format!("{API}/api/connection/list"))?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}

fn fetch_databases(connection_id: &str) -> Result<Vec<String>, String> {
    let value = get_json(&format!(
        "{API}/api/schema/databases?connection_id={}",
        encode(connection_id)
    ))?;
    Ok(value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(|name| name.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

fn fetch_schemas(connection_id: &str, database: &str) -> Result<Vec<String>, String> {
    let value = get_json(&format!(
        "{API}/api/schema/schemas?connection_id={}&database={}",
        encode(connection_id),
        encode(database)
    ))?;
    Ok(value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

fn fetch_tables(connection_id: &str, database: &str, schema: &str) -> Result<Vec<String>, String> {
    let value = get_json(&format!(
        "{API}/api/schema/tables?connection_id={}&database={}&schema={}",
        encode(connection_id),
        encode(database),
        encode(schema)
    ))?;
    Ok(value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(|name| name.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

struct TableData {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn fetch_rows(
    connection: &Connection,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<TableData, String> {
    let sql = format!("SELECT * FROM {} LIMIT {ROW_LIMIT}", connection.quote(table));
    run_sql(connection, database, schema, &sql)
}

fn run_sql(
    connection: &Connection,
    database: &str,
    schema: &str,
    sql: &str,
) -> Result<TableData, String> {
    let value = post_json(
        &format!("{API}/api/query/execute"),
        &json!({
            "connectionId": connection.id,
            "database": database,
            "schema": schema,
            "sql": sql,
            "maxRows": ROW_LIMIT,
        }),
    )?;
    let columns = value
        .get("columns")
        .and_then(|columns| columns.as_array())
        .map(|columns| {
            columns
                .iter()
                .filter_map(|name| name.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let rows = value
        .get("rows")
        .and_then(|rows| rows.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .map(|cells| cells.iter().map(cell_text).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(error) = value.get("error").and_then(|error| error.as_object()) {
        let message = error
            .get("detail")
            .or_else(|| error.get("message"))
            .and_then(|message| message.as_str())
            .unwrap_or("查询失败");
        return Err(message.to_string());
    }
    Ok(TableData { columns, rows })
}

fn cell_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}


// ---------------------------------------------------------------- grid

/// Backing store handed to gpui-kit"s DataTable: virtual scrolling, resizable
/// columns, sorting, row/cell selection all come from the component.
struct GridDelegate {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl TableDelegate for GridDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, index: usize, _: &App) -> Column {
        let name = self.columns.get(index).cloned().unwrap_or_default();
        Column::new(name.clone(), name)
    }

    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let text = self
            .rows
            .get(row)
            .and_then(|cells| cells.get(col))
            .cloned()
            .unwrap_or_default();
        let is_null = text == "NULL";
        let muted = cx.theme().colors.muted_foreground;
        div()
            .truncate()
            .when(is_null, |this| this.text_color(muted))
            .child(text)
    }
}

// ---------------------------------------------------------------- state

#[derive(Clone)]
struct Tab {
    key: String,
    title: String,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
}

struct Omdbx {
    connections: Vec<Connection>,
    error: Option<String>,
    open_connections: HashSet<String>,
    databases: HashMap<String, Vec<String>>,
    open_databases: HashSet<String>,
    tables: HashMap<String, Vec<String>>,
    tabs: Vec<Tab>,
    active: Option<usize>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    loading: bool,
    grid: Option<Entity<TableState<GridDelegate>>>,
    query: Option<Entity<EditorState>>,
    show_query: bool,
}

impl Omdbx {
    fn new() -> Self {
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
        Self {
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
            grid: None,
            query: None,
            show_query: false,
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        match fetch_connections() {
            Ok(list) => {
                self.connections = list;
                self.error = None;
                self.databases.clear();
                self.tables.clear();
            }
            Err(message) => self.error = Some(message),
        }
        cx.notify();
    }

    fn toggle_connection(&mut self, id: &str, cx: &mut Context<Self>) {
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

    fn toggle_database(&mut self, connection_id: &str, database: &str, cx: &mut Context<Self>) {
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

    fn open_table(
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
        if let Some(index) = self.tabs.iter().position(|tab| tab.key == key) {
            self.active = Some(index);
        } else {
            self.tabs.push(Tab {
                key: key.clone(),
                title: table.to_string(),
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema,
                table: table.to_string(),
            });
            self.active = Some(self.tabs.len() - 1);
        }
        self.load_active(window, cx);
    }

    fn load_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        self.loading = true;
        cx.notify();
        match fetch_rows(&connection, &tab.database, &tab.schema, &tab.table) {
            Ok(data) => {
                let delegate = GridDelegate {
                    columns: data.columns.clone(),
                    rows: data.rows.clone(),
                };
                self.grid = Some(cx.new(|cx| TableState::new(delegate, window, cx)));
                self.columns = data.columns;
                self.rows = data.rows;
                self.error = None;
            }
            Err(message) => {
                self.grid = None;
                self.columns = Vec::new();
                self.rows = Vec::new();
                self.error = Some(message);
            }
        }
        self.loading = false;
        cx.notify();
    }


    fn open_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.query.is_none() {
            self.query = Some(cx.new(|cx| {
                EditorState::new(window, cx)
                    .default_value(
                        "SELECT id, name\nFROM customers\nWHERE id > 10\nORDER BY id\nLIMIT 50",
                    )
                    .language("sql")
            }));
        }
        self.show_query = true;
        cx.notify();
    }

    /// Connection the query runs against: the active table tab, else the first connection.
    fn query_target(&self) -> Option<(Connection, String, String)> {
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

    fn run_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.query.clone() else { return };
        let sql = state.read(cx).value().to_string();
        if sql.trim().is_empty() {
            return;
        }
        let Some((connection, database, schema)) = self.query_target() else {
            self.error = Some("没有可用的连接".to_string());
            cx.notify();
            return;
        };
        self.loading = true;
        cx.notify();
        match run_sql(&connection, &database, &schema, &sql) {
            Ok(data) => {
                let delegate = GridDelegate {
                    columns: data.columns.clone(),
                    rows: data.rows.clone(),
                };
                self.grid = Some(cx.new(|cx| TableState::new(delegate, window, cx)));
                self.columns = data.columns;
                self.rows = data.rows;
                self.error = None;
            }
            Err(message) => {
                self.grid = None;
                self.columns = Vec::new();
                self.rows = Vec::new();
                self.error = Some(message);
            }
        }
        self.loading = false;
        cx.notify();
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active = None;
            self.grid = None;
            self.columns.clear();
            self.rows.clear();
        } else {
            let next = index.min(self.tabs.len() - 1);
            self.active = Some(next);
            self.load_active(window, cx);
            return;
        }
        cx.notify();
    }
}

/// One sidebar tree row: indent, chevron/icon, name, hover.
fn tree_row(
    id: String,
    indent: f32,
    icon: Icon,
    name: String,
    colors: gpui_kit::component::ThemeColor,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(SharedString::from(id))
        .h(px(28.))
        .mx_2()
        .pl(px(indent))
        .pr_2()
        .gap_1()
        .items_center()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .hover(|style| style.bg(colors.secondary))
        .child(icon.xsmall().text_color(colors.muted_foreground))
        .child(div().min_w_0().truncate().child(name))
        .child(div().flex_1())
        .on_click(on_click)
}

// ---------------------------------------------------------------- view

impl Omdbx {
    fn connection_row(&self, connection: &Connection, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.open_connections.contains(&connection.id);
        let id = connection.id.clone();
        let colors = cx.theme().colors;
        tree_row(
            format!("conn-{}", connection.id),
            8.0,
            Icon::new(if is_open {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            }),
            connection.name.clone(),
            colors,
            cx.listener(move |this, _, _, cx| this.toggle_connection(&id, cx)),
        )
        .child(
            div()
                .text_xs()
                .text_color(colors.muted_foreground)
                .child(connection.db_type.clone()),
        )
    }


    fn query_pane(&self, tab: &Option<Tab>, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let target = self
            .query_target()
            .map(|(connection, database, _)| format!("{} · {}", connection.name, database))
            .unwrap_or_else(|| "没有连接".to_string());

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .child(
                v_flex()
                    .h(px(240.))
                    .flex_none()
                    .border_b_1()
                    .border_color(colors.border)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(13.))
                    .child(match self.query.as_ref() {
                        Some(state) => Editor::new(state).h(gpui_kit::relative(1.)).into_any_element(),
                        None => div().into_any_element(),
                    }),
            )
            .child(
                h_flex()
                    .h(px(40.))
                    .flex_none()
                    .px_3()
                    .gap_2()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        Button::new("run-query")
                            .primary()
                            .small()
                            .label("运行")
                            .on_click(cx.listener(|this, _, window, cx| this.run_query(window, cx))),
                    )
                    .child(div().text_xs().text_color(muted).child(target))
                    .child(div().flex_1())
                    .child(
                        div().text_xs().text_color(muted).child(if self.loading {
                            "执行中…".to_string()
                        } else {
                            match &self.error {
                                Some(message) => message.clone(),
                                None => format!("{} 行", self.rows.len()),
                            }
                        }),
                    ),
            )
            .child(match self.grid.clone() {
                Some(grid) => div()
                    .flex_1()
                    .min_h_0()
                    .child(DataTable::new(&grid).stripe(true))
                    .into_any_element(),
                None => v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(format!(
                                "在编辑器里写 SQL，⌘↵ 或点运行{}",
                                match tab {
                                    Some(tab) => format!("（当前表 {}）", tab.title),
                                    None => String::new(),
                                }
                            )),
                    )
                    .into_any_element(),
            })
    }

    fn main_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let tab = self.active.and_then(|index| self.tabs.get(index)).cloned();
        let muted = colors.muted_foreground;

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            // tab strip
            .child(
                h_flex()
                    .h(px(40.))
                    .flex_none()
                    .px_2()
                    .gap_1()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .children(self.tabs.iter().enumerate().map(|(index, item)| {
                        let is_active = Some(index) == self.active;
                        let title = item.title.clone();
                        let close_index = index;
                        h_flex()
                            .id(SharedString::from(format!("tab-{}", item.key)))
                            .h(px(28.))
                            .px_2()
                            .gap_2()
                            .rounded_md()
                            .cursor_pointer()
                            .text_sm()
                            .when(is_active, |this| {
                                this.bg(colors.secondary).text_color(colors.foreground)
                            })
                            .when(!is_active, |this| {
                                this.text_color(muted)
                                    .hover(|style| style.bg(colors.secondary))
                            })
                            .child(title)
                            .child(
                                div()
                                    .id(SharedString::from(format!("close-{}", item.key)))
                                    .px_1()
                                    .rounded_sm()
                                    .text_xs()
                                    .text_color(muted)
                                    .hover(|style| style.bg(colors.border))
                                    .child("×")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.close_tab(close_index, window, cx);
                                    })),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.show_query = false;
                                this.active = Some(index);
                                this.load_active(window, cx);
                            }))
                    }))
                    .children(self.query.as_ref().map(|_| {
                        let is_active = self.show_query;
                        h_flex()
                            .id("tab-query")
                            .h(px(28.))
                            .px_2()
                            .rounded_md()
                            .cursor_pointer()
                            .text_sm()
                            .when(is_active, |this| this.bg(colors.secondary).text_color(colors.foreground))
                            .when(!is_active, |this| this.text_color(muted).hover(|style| style.bg(colors.secondary)))
                            .child("查询")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_query = true;
                                cx.notify();
                            }))
                    }))
                    .child(
                        Button::new("new-query")
                            .ghost()
                            .xsmall()
                            .label("+")
                            .on_click(cx.listener(|this, _, window, cx| this.open_query(window, cx))),
                    ),
            )
            // toolbar
            .child(
                h_flex()
                    .h(px(40.))
                    .flex_none()
                    .px_4()
                    .gap_2()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_sm()
                    .child(match &tab {
                        Some(tab) => tab.title.clone(),
                        None => "选择一张表".to_string(),
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(match &tab {
                                Some(tab) => format!(
                                    "{} · {}",
                                    self.connections
                                        .iter()
                                        .find(|item| item.id == tab.connection_id)
                                        .map(|item| item.name.clone())
                                        .unwrap_or_default(),
                                    tab.database
                                ),
                                None => String::new(),
                            }),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(if self.loading {
                                "加载中…".to_string()
                            } else if tab.is_some() {
                                format!("{} 行", self.rows.len())
                            } else {
                                String::new()
                            }),
                    ),
            )
            // body
            .child(if self.show_query {
                self.query_pane(&tab, cx).into_any_element()
            } else {
                match &tab {
                None => v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(div().text_lg().child("GPUI Kit · P1"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(match &self.error {
                                Some(message) => format!("读取失败：{message}"),
                                None => "在左侧展开连接和数据库，点表名打开".to_string(),
                            }),
                    )
                    .into_any_element(),
                Some(_) if self.columns.is_empty() && self.rows.is_empty() => v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(match &self.error {
                                Some(message) => format!("查询失败：{message}"),
                                None => "没有数据".to_string(),
                            }),
                    )
                    .into_any_element(),
                Some(_) => match self.grid.clone() {
                    Some(grid) => div()
                        .flex_1()
                        .min_h_0()
                        .child(DataTable::new(&grid).stripe(true))
                        .into_any_element(),
                    None => v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_sm()
                                .text_color(muted)
                                .child(match &self.error {
                                    Some(message) => format!("查询失败：{message}"),
                                    None => "没有数据".to_string(),
                                }),
                        )
                        .into_any_element(),
                }
                }
            })
    }
}

impl Render for Omdbx {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;

        // sidebar tree rows, built up front so the closure below stays simple
        let mut tree: Vec<gpui_kit::AnyElement> = Vec::new();
        for connection in &self.connections {
            tree.push(self.connection_row(connection, cx).into_any_element());
            if !self.open_connections.contains(&connection.id) {
                continue;
            }
            for database in self.databases.get(&connection.id).cloned().unwrap_or_default() {
                let db_open = self
                    .open_databases
                    .contains(&format!("{}|{}", connection.id, database));
                let conn_id = connection.id.clone();
                let db_name = database.clone();
                tree.push(
                    tree_row(
                        format!("db-{}|{}", connection.id, database),
                        26.0,
                        Icon::new(if db_open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        }),
                        database.clone(),
                        colors,
                        cx.listener(move |this, _, _, cx| {
                            this.toggle_database(&conn_id, &db_name, cx)
                        }),
                    )
                    .into_any_element(),
                );
                if !db_open {
                    continue;
                }
                for table in self
                    .tables
                    .get(&format!("{}|{}", connection.id, database))
                    .cloned()
                    .unwrap_or_default()
                {
                    let conn_id = connection.id.clone();
                    let db_name = database.clone();
                    let table_name = table.clone();
                    tree.push(
                        tree_row(
                            format!("t-{}|{}|{}", connection.id, database, table),
                            48.0,
                            Icon::new(AssetIcon::Grid2x2),
                            table.clone(),
                            colors,
                            cx.listener(move |this, _, window, cx| {
                                this.open_table(&conn_id, &db_name, &table_name, window, cx)
                            }),
                        )
                        .into_any_element(),
                    );
                }
            }
        }

        h_flex()
            .size_full()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(
                v_flex()
                    .w(px(248.))
                    .h_full()
                    .flex_none()
                    .bg(colors.sidebar)
                    .border_r_1()
                    .border_color(colors.sidebar_border)
                    .child(
                        h_flex()
                            .h(px(44.))
                            .flex_none()
                            .px_3()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child(format!("我的连接  {}", self.connections.len())),
                            )
                            .child(
                                Button::new("refresh")
                                    .ghost()
                                    .xsmall()
                                    .label("刷新")
                                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .py_1()
                            .children(tree),
                    ),
            )
            .child(self.main_pane(cx))
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
        gpui_kit::init(cx);
        Theme::update(cx, |theme| theme.mode = ThemeMode::Light);
        gpui_kit::open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::centered(
                    None,
                    gpui_kit::size(px(1100.), px(700.)),
                    cx,
                ))),
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(|_| Omdbx::new()),
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
