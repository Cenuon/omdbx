//! OMDBX desktop, Rust + GPUI, styled with GPUI Kit.
//! Talks to the existing `dbx-web` sidecar over HTTP so all connection and query
//! logic is shared with the Electron build.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Editor, EditorState, Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::Selectable;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable, Theme, ThemeMode, TitleBar,
};
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::base::{h_resizable, resizable_panel, ResizeHandleRenderer};
use gpui_kit::prelude::*;
use gpui_kit::{
    div, px, App, Context, Entity, FocusHandle, FontFeatures, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, SharedString, Styled,
    Window, WindowOptions,
};
use serde::Deserialize;
use serde_json::json;

gpui_kit::actions!(omdbx, [RefreshConnections]);

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
    driver_profile: Option<String>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdateLevel {
    Patch,
    Minor,
    Major,
}

#[derive(Clone)]
struct UpdateInfo {
    version: String,
    level: UpdateLevel,
}

impl UpdateLevel {
    fn color(self) -> Hsla {
        match self {
            UpdateLevel::Patch => gpui_kit::rgb(0x3F7FD0).into(),
            UpdateLevel::Minor => gpui_kit::rgb(0xE0A83C).into(),
            UpdateLevel::Major => gpui_kit::rgb(0xE5534B).into(),
        }
    }
}

fn parse_version(text: &str) -> (u32, u32, u32) {
    let Some(start) = text.find(|c: char| c.is_ascii_digit()) else {
        return (0, 0, 0);
    };
    let digits: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = digits.split('.').filter_map(|part| part.parse::<u32>().ok());
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn compare_versions(current: &str, latest: &str) -> Option<UpdateLevel> {
    let (current_major, current_minor, current_patch) = parse_version(current);
    let (latest_major, latest_minor, latest_patch) = parse_version(latest);
    if latest_major > current_major {
        Some(UpdateLevel::Major)
    } else if latest_major == current_major && latest_minor > current_minor {
        Some(UpdateLevel::Minor)
    } else if latest_major == current_major
        && latest_minor == current_minor
        && latest_patch > current_patch
    {
        Some(UpdateLevel::Patch)
    } else {
        None
    }
}

fn fetch_update() -> Option<UpdateInfo> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(5));
    // Follow the same HTTPS proxy as git/push without picking up all_proxy,
    // which may be a SOCKS endpoint this build does not support.
    if let Some(proxy) = std::env::var("https_proxy")
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .ok()
        .and_then(|value| ureq::Proxy::new(&value).ok())
    {
        builder = builder.proxy(proxy);
    }
    let agent = builder.build();
    // The releases/latest redirect names the newest tag and is not subject to
    // the unauthenticated API rate limit.
    let response = agent
        .get("https://github.com/Cenuon/omdbx/releases/latest")
        .call()
        .ok()?;
    let final_url = response.get_url().to_string();
    let tag = final_url.rsplit("/tag/").next()?;
    if tag == final_url {
        return None;
    }
    let latest = tag.trim_start_matches("desktop-v").trim_start_matches('v');
    let level = compare_versions(env!("CARGO_PKG_VERSION"), latest)?;
    Some(UpdateInfo {
        version: latest.to_string(),
        level,
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ColumnMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    data_type: String,
    #[serde(default)]
    comment: String,
}

fn fetch_columns(
    connection_id: &str,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<Vec<ColumnMeta>, String> {
    let value = get_json(&format!(
        "{API}/api/schema/columns?connection_id={}&database={}&schema={}&table={}",
        encode(connection_id),
        encode(database),
        encode(schema),
        encode(table)
    ))?;
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
    column_types: Vec<String>,
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
    let column_types = value
        .get("column_types")
        .and_then(|types| types.as_array())
        .map(|types| {
            types
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
    Ok(TableData { columns, column_types, rows })
}

fn sql_tables(sql: &str) -> Vec<String> {
    let tokens: Vec<&str> = sql.split_whitespace().collect();
    let mut tables: Vec<String> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let keyword = token
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_ascii_uppercase();
        if keyword != "FROM" && keyword != "JOIN" {
            continue;
        }
        let Some(next) = tokens.get(index + 1) else {
            continue;
        };
        let next = next.trim_matches(|c: char| {
            matches!(c, ',' | ';' | '(' | ')' | '\'' | '"' | '`' | '[' | ']')
        });
        let table = next
            .rsplit('.')
            .next()
            .unwrap_or(next)
            .trim_matches(|c| matches!(c, '`' | '"' | '[' | ']'))
            .to_string();
        if !table.is_empty() && !tables.contains(&table) {
            tables.push(table);
        }
    }
    tables
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
    meta: Vec<ColumnMeta>,
    show_comments: bool,
}

impl TableDelegate for GridDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len() + 1
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, index: usize, _: &App) -> Column {
        // right padding only: the cell origin is the column boundary, so the
        // rule can be drawn exactly on it; the header's sort-icon correction
        // is zero because the right padding matches the default
        let padding = gpui_kit::Edges {
            top: px(0.),
            right: px(12.),
            bottom: px(0.),
            left: px(0.),
        };
        if index == 0 {
            return Column::new("#", "#")
                .width(px(52.))
                .fixed_left()
                .resizable(false)
                .movable(false)
                .selectable(false)
                .paddings(padding);
        }
        let name = self.columns.get(index - 1).cloned().unwrap_or_default();
        let width = self
            .meta
            .get(index - 1)
            .map(|meta| {
                let text = (meta.name.chars().count() as f32 * 8.)
                    .max(meta.data_type.chars().count() as f32 * 7.)
                    .max(meta.comment.chars().count() as f32 * 11.);
                (text + 28.).clamp(120., 260.)
            })
            .unwrap_or(140.);
        Column::new(name.clone(), name)
            .width(px(width))
            .paddings(padding)
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let muted = cx.theme().colors.muted_foreground;
        if col_ix == 0 {
            return div()
                .h_full()
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .text_size(px(11.))
                .text_color(muted)
                .child("#")
                .into_any_element();
        }
        let name = self.columns.get(col_ix - 1).cloned().unwrap_or_default();
        let (data_type, comment) = self
            .meta
            .get(col_ix - 1)
            .map(|meta| (meta.data_type.clone(), meta.comment.clone()))
            .unwrap_or_default();
        let type_color = data_type_color(&data_type, muted);
        div()
            .h_full()
            .w_full()
            .flex()
            .items_center()
            .when(col_ix > 1, |this| {
                this.child(
                    div()
                        .h_full()
                        .w(px(1.))
                        .flex_none()
                        .bg(cx.theme().table_row_border),
                )
                .child(div().w(px(11.)))
            })
            .when(col_ix == 1, |this| this.child(div().w(px(12.))))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(1.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(name),
                    )
                    .when(!data_type.is_empty(), |this| {
                        this.child(
                            div()
                                .truncate()
                                .text_size(px(10.))
                                .text_color(type_color)
                                .child(data_type),
                        )
                    })
                    .when(self.show_comments && !comment.is_empty(), |this| {
                        this.child(
                            div()
                                .truncate()
                                .text_size(px(10.))
                                .text_color(gpui_kit::rgb(0x9A9AA0))
                                .child(comment),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let muted = cx.theme().colors.muted_foreground;
        if col == 0 {
            return div()
                .h_full()
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .text_size(px(11.))
                .text_color(muted)
                .font_features(tabular())
                .child(format!("{}", row + 1))
                .into_any_element();
        }
        let text = self
            .rows
            .get(row)
            .and_then(|cells| cells.get(col - 1))
            .cloned()
            .unwrap_or_default();
        let is_null = text == "NULL";
        let numeric = self
            .meta
            .get(col - 1)
            .map(|meta| is_numeric_type(&meta.data_type))
            .unwrap_or(false);
        div()
            .h_full()
            .w_full()
            .flex()
            .items_center()
            .when(col > 1, |this| {
                this.child(
                    div()
                        .h_full()
                        .w(px(1.))
                        .flex_none()
                        .bg(cx.theme().table_row_border),
                )
                .child(div().w(px(11.)))
            })
            .when(col == 1, |this| this.child(div().w(px(12.))))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .text_size(px(13.))
                    .when(numeric, |this| this.text_right().font_features(tabular()))
                    .when(is_null, |this| this.text_color(muted))
                    .child(text),
            )
            .into_any_element()
    }
}

fn data_type_color(data_type: &str, muted: Hsla) -> Hsla {
    let ty = data_type.to_ascii_lowercase();
    if ["int", "decimal", "numeric", "float", "double", "real", "bit"]
        .iter()
        .any(|needle| ty.contains(needle))
    {
        gpui_kit::rgb(0x6B9BD8).into()
    } else if ["char", "text", "enum", "set", "json", "blob", "binary"]
        .iter()
        .any(|needle| ty.contains(needle))
    {
        gpui_kit::rgb(0x63B3A6).into()
    } else if ["date", "time", "year"]
        .iter()
        .any(|needle| ty.contains(needle))
    {
        gpui_kit::rgb(0xC9A15E).into()
    } else {
        muted
    }
}

fn is_numeric_type(data_type: &str) -> bool {
    let ty = data_type.to_ascii_lowercase();
    ["int", "decimal", "numeric", "float", "double", "real", "bit"]
        .iter()
        .any(|needle| ty.contains(needle))
}

fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

// ---------------------------------------------------------------- state

#[derive(Clone)]
struct QueryTab {
    key: String,
    title: String,
    editor: Entity<EditorState>,
}

#[derive(Clone, Copy)]
enum CloseTarget {
    Table(usize),
    Query(usize),
}

#[derive(Clone)]
struct Tab {
    key: String,
    title: String,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
}

/// What each flat tree row represents, used to pin ancestors while scrolling.
enum TreeEntry {
    Connection(String),
    Database(String, String),
    Table(String, String),
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
    queries: Vec<QueryTab>,
    active_query: Option<usize>,
    next_query: u64,
    show_query: bool,
    search: Entity<InputState>,
    filter: String,
    sidebar_open: bool,
    notice: Option<String>,
    notice_id: u64,
    tools_open: bool,
    query_ms: Option<u128>,
    column_meta: Vec<ColumnMeta>,
    show_comments: bool,
    update: Option<UpdateInfo>,
    search_open: bool,
    focus_handle: FocusHandle,
    tree_scroll: ScrollHandle,
}

impl Omdbx {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索连接或表"));
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
            grid: None,
            queries: Vec::new(),
            active_query: None,
            next_query: 0,
            show_query: false,
            search,
            filter: String::new(),
            sidebar_open: true,
            notice: None,
            notice_id: 0,
            tools_open: false,
            query_ms: None,
            column_meta: Vec::new(),
            show_comments: true,
            update: None,
            search_open: false,
            focus_handle: cx.focus_handle(),
            tree_scroll: ScrollHandle::new(),
        };
        this.check_for_update(cx);
        this
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

    /// Reveal the active table in the tree: expand its connection and database,
    /// load anything missing, then scroll the row into view. No active table
    /// means no-op.
    fn locate_active_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    fn tree_index_of_table(&self, tab: &Tab) -> Option<usize> {
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
                let table_meta = fetch_columns(
                    &tab.connection_id,
                    &tab.database,
                    &tab.schema,
                    &tab.table,
                )
                .unwrap_or_default();
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
                self.column_meta = meta.clone();
                let delegate = GridDelegate {
                    columns: data.columns.clone(),
                    rows: data.rows.clone(),
                    meta,
                    show_comments: self.show_comments,
                };
                self.grid = Some(cx.new(|cx| TableState::new(delegate, window, cx).col_resizable(false)));
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


    fn check_for_update(&self, cx: &mut Context<Self>) {
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

    /// Section header above the connection tree: title on the left, search and
    /// connection actions on the right. Clicking search swaps the whole row for
    /// an input, matching the reference layout.
    fn connections_header(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
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
            .dropdown_menu_with_anchor(gpui_kit::Anchor::TopRight, move |menu, _, _| {
                let menu = menu
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
            .child(Self::icon_button(
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
            .child(Self::icon_button(
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

    fn show_notice(&mut self, message: &str, cx: &mut Context<Self>) {
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

    fn open_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.next_query += 1;
        let number = self.next_query;
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
        });
        self.active_query = Some(self.queries.len() - 1);
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
        let Some((connection, database, schema)) = self.query_target() else {
            self.error = Some("没有可用的连接".to_string());
            cx.notify();
            return;
        };
        self.loading = true;
        cx.notify();
        let started = std::time::Instant::now();
        match run_sql(&connection, &database, &schema, &sql) {
            Ok(data) => {
                let mut schema_columns: Vec<ColumnMeta> = Vec::new();
                for table in sql_tables(&sql) {
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
                            column.data_type =
                                data.column_types.get(index).cloned().unwrap_or_default();
                        }
                        column
                    })
                    .collect();
                self.column_meta = meta.clone();
                let delegate = GridDelegate {
                    columns: data.columns.clone(),
                    rows: data.rows.clone(),
                    meta,
                    show_comments: self.show_comments,
                };
                self.grid = Some(cx.new(|cx| TableState::new(delegate, window, cx).col_resizable(false)));
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
        self.query_ms = Some(started.elapsed().as_millis());
        self.loading = false;
        cx.notify();
    }

    fn close_query(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.queries.len() {
            return;
        }
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

macro_rules! db_icon {
    ($name:literal) => {
        include_bytes!(concat!("../assets/db-icons/", $name, ".svg")) as &[u8]
    };
}

fn connection_color(key: &str) -> Hsla {
    match key {
        "mysql" => gpui_kit::rgb(0x0A5AF6),
        "mariadb" => gpui_kit::rgb(0x1A6B8A),
        "tidb" => gpui_kit::rgb(0xDC150B),
        "oceanbase" => gpui_kit::rgb(0x1F6FEB),
        "postgres" => gpui_kit::rgb(0x326690),
        "redis" => gpui_kit::rgb(0xD82C20),
        "sqlite" => gpui_kit::rgb(0x0F80CC),
        "sqlserver" => gpui_kit::rgb(0xCC2927),
        "oracle" => gpui_kit::rgb(0xC74634),
        "mongodb" => gpui_kit::rgb(0x47A248),
        "clickhouse" => gpui_kit::rgb(0xC7A500),
        _ => gpui_kit::rgb(0x5B8DEF),
    }
    .into()
}

fn connection_icon(key: &str, active: bool) -> Icon {
    let color = if active {
        connection_color(key)
    } else {
        gpui_kit::rgb(0x8C8C91).into()
    };
    let bytes: &[u8] = match key {
        "mysql" => db_icon!("mysql"),
        "mariadb" => db_icon!("mariadb"),
        "tidb" => db_icon!("tidb"),
        "oceanbase" => db_icon!("oceanbase"),
        "postgres" => db_icon!("postgres"),
        "redis" => db_icon!("redis"),
        "sqlite" => db_icon!("sqlite"),
        "sqlserver" => db_icon!("sqlserver"),
        "oracle" => db_icon!("oracle"),
        "mongodb" => db_icon!("mongodb"),
        "clickhouse" => db_icon!("clickhouse"),
        _ => {
            return Icon::new(AssetIcon::Database)
                .with_size(px(16.))
                .text_color(color);
        }
    };
    Icon::default().data(bytes).with_size(px(16.)).text_color(color)
}

/// The tools row doubles as a popover trigger; the popover only needs
/// `Selectable`, and the row carries all of the visual state.
struct ToolsTrigger(gpui_kit::Stateful<gpui_kit::Div>);

impl Selectable for ToolsTrigger {
    fn selected(self, _: bool) -> Self {
        self
    }

    fn is_selected(&self) -> bool {
        false
    }
}

impl IntoElement for ToolsTrigger {
    type Element = <gpui_kit::Stateful<gpui_kit::Div> as IntoElement>::Element;

    fn into_element(self) -> Self::Element {
        self.0.into_element()
    }
}

fn tool_menu_row(
    id: &'static str,
    icon: AssetIcon,
    label: &'static str,
    notice: &'static str,
    weak: gpui_kit::WeakEntity<Omdbx>,
    colors: gpui_kit::component::ThemeColor,
) -> gpui_kit::AnyElement {
    tree_row(
        id.to_string(),
        8.,
        Icon::new(icon),
        label.to_string(),
        colors,
        move |_, _, cx| {
            let _ = weak.update(cx, |this, cx| {
                this.tools_open = false;
                this.show_notice(notice, cx);
            });
        },
    )
    .mx(px(0.))
    .into_any_element()
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
        .h(px(32.))
        .flex_none()
        .mx_2()
        .pl(px(indent))
        .pr_2()
        .gap_2()
        .items_center()
        .rounded_md()
        .cursor_pointer()
        .text_size(px(13.))
        .hover(|style| style.bg(colors.sidebar_accent))
        .child(icon.with_size(px(16.)).text_color(colors.muted_foreground))
        .child(div().min_w_0().truncate().child(name))
        .child(div().flex_1())
        .on_click(on_click)
}

// ---------------------------------------------------------------- view

impl Omdbx {
    fn connection_row(&self, connection: &Connection, cx: &mut Context<Self>) -> impl IntoElement {
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
        h_flex()
            .id(SharedString::from(format!("conn-{}", connection.id)))
            .h(px(32.))
            .flex_none()
            .mx_2()
            .pl(px(12.))
            .pr_2()
            .gap_2()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .text_size(px(13.))
            .hover(|style| style.bg(colors.sidebar_accent))
            .child(connection_icon(&key, active))
            .child(div().min_w_0().truncate().child(connection.name.clone()))
            .child(div().flex_1())
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_connection(&id, cx)))
    }

    fn database_row(
        &self,
        connection_id: &str,
        database: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme().colors;
        let conn_id = connection_id.to_string();
        let db_name = database.to_string();
        tree_row(
            format!("db-{connection_id}|{database}"),
            28.0,
            Icon::new(AssetIcon::Database),
            database.to_string(),
            colors,
            cx.listener(move |this, _, _, cx| this.toggle_database(&conn_id, &db_name, cx)),
        )
    }

    /// Sidebar tree, narrowed to rows matching the search field. While a query
    /// is active the matching branches are shown expanded and every other row
    /// is dropped, so the tree reads as a flat result list.
    fn tree(
        &self,
        cx: &mut Context<Self>,
    ) -> (Vec<gpui_kit::AnyElement>, Vec<TreeEntry>) {
        let colors = cx.theme().colors;
        let filter = self.filter.trim().to_lowercase();
        let searching = !filter.is_empty();
        let matches = |text: &str| filter.is_empty() || text.to_lowercase().contains(&filter);

        let mut rows = Vec::new();
        let mut entries = Vec::new();
        for connection in &self.connections {
            let conn_match = matches(&connection.name) || matches(&connection.db_type);
            let mut children = Vec::new();
            let mut child_entries = Vec::new();

            for database in self
                .databases
                .get(&connection.id)
                .cloned()
                .unwrap_or_default()
            {
                let key = format!("{}|{}", connection.id, database);
                let db_match = matches(&database);
                let tables = self.tables.get(&key).cloned().unwrap_or_default();
                let any_table_match = tables.iter().any(|table| matches(table));
                if searching && !conn_match && !db_match && !any_table_match {
                    continue;
                }

                let db_open = self.open_databases.contains(&key) || searching;
                children.push(self.database_row(&connection.id, &database, cx).into_any_element());
                child_entries.push(TreeEntry::Database(
                    connection.id.clone(),
                    database.clone(),
                ));
                if !db_open {
                    continue;
                }

                for table in tables {
                    if searching && !conn_match && !db_match && !matches(&table) {
                        continue;
                    }
                    let conn_id = connection.id.clone();
                    let db_name = database.clone();
                    let table_name = table.clone();
                    children.push(
                        tree_row(
                            format!("t-{}|{}|{}", connection.id, database, table),
                            44.0,
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
                continue;
            }
            rows.push(self.connection_row(connection, cx).into_any_element());
            entries.push(TreeEntry::Connection(connection.id.clone()));
            if self.open_connections.contains(&connection.id) || searching {
                rows.extend(children);
                entries.extend(child_entries);
            }
        }
        (rows, entries)
    }

    /// Rows pinned above the scrolling tree: the open connection, plus its
    /// database when the visible list is inside that database.
    fn sticky_rows(
        &self,
        entries: &[TreeEntry],
        cx: &mut Context<Self>,
    ) -> Vec<gpui_kit::AnyElement> {
        if !self.filter.trim().is_empty() || self.tree_scroll.offset().y >= px(0.) {
            return Vec::new();
        }
        let (connection_id, database) = match entries.get(self.tree_scroll.top_item()) {
            Some(TreeEntry::Connection(connection_id)) => (connection_id, None),
            Some(TreeEntry::Database(connection_id, database)) => (connection_id, Some(database)),
            Some(TreeEntry::Table(connection_id, database)) => (connection_id, Some(database)),
            None => return Vec::new(),
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| &connection.id == connection_id)
        else {
            return Vec::new();
        };
        let mut rows = vec![self.connection_row(connection, cx).into_any_element()];
        if let Some(database) = database {
            rows.push(self.database_row(connection_id, database, cx).into_any_element());
        }
        rows
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        v_flex()
            .w_full()
            .h_full()
            .bg(colors.sidebar)
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
                        cx.listener(|this, _, _, cx| this.show_notice("连接向导未完成", cx)),
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
                            .trigger(ToolsTrigger(
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
                let sticky = self.sticky_rows(&tree_entries, cx);
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
                                .bg(colors.sidebar)
                                .border_b_1()
                                .border_color(colors.border.opacity(0.4))
                                .shadow_sm()
                                .children(sticky),
                        )
                    })
            })
            .child(
                h_flex()
                    .h(px(44.))
                    .flex_none()
                    .px_3()
                    .items_center()
                    .justify_between()
                    .child(Self::icon_button(
                        "settings",
                        Icon::new(AssetIcon::Settings)
                            .with_size(px(16.))
                            .text_color(colors.muted_foreground)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, _, cx| this.show_notice("设置未完成", cx)),
                    ))
                    // no update, no icon
                    .when_some(self.update.clone(), |this, update| {
                        let color = update.level.color();
                        this.child(Self::icon_button(
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

    /// Table with an outer frame, a small gap, and a rounded content area.
    fn table_frame(
        &self,
        grid: &Entity<TableState<GridDelegate>>,
        colors: gpui_kit::component::ThemeColor,
    ) -> impl IntoElement {
        let header_height = self.header_height();
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
                    .child(DataTable::new(grid).bordered(false).large().stripe(true).header_row_height(header_height)),
            )
    }

    fn empty_state(
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

    fn header_height(&self) -> gpui_kit::Pixels {
        if self.show_comments
            && self
                .column_meta
                .iter()
                .any(|meta| !meta.comment.is_empty())
        {
            px(64.)
        } else {
            px(48.)
        }
    }

    fn query_crumb(
        label: String,
        colors: gpui_kit::component::ThemeColor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let muted = colors.muted_foreground;
        h_flex()
            .id(SharedString::from(format!("crumb-{label}")))
            .h(px(28.))
            .px_1()
            .gap_1()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .hover(|style| style.bg(colors.sidebar_accent))
            .child(div().text_size(px(13.)).child(label))
            .child(Icon::new(IconName::ChevronDown).xsmall().text_color(muted))
            .on_click(cx.listener(|this, _, _, cx| this.show_notice("选择未完成", cx)))
    }

    fn query_pane(&self, _tab: &Option<Tab>, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let (connection_name, database, schema) = self
            .query_target()
            .map(|(connection, database, schema)| (connection.name, database, schema))
            .unwrap_or_else(|| ("未连接".to_string(), "main".to_string(), "main".to_string()));
        let editor = self
            .active_query
            .and_then(|index| self.queries.get(index))
            .map(|query| query.editor.clone());
        let rows = self.rows.len();
        let elapsed = if self.loading {
            "执行中…".to_string()
        } else {
            self.query_ms
                .map(|ms| format!("{ms} ms"))
                .unwrap_or_else(|| "—".to_string())
        };

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            // connection, database and schema pickers, then the actions
            .child(
                h_flex()
                    .h(px(48.))
                    .flex_none()
                    .px_3()
                    .gap_1()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        h_flex()
                            .id("query-connection")
                            .h(px(28.))
                            .px_2()
                            .gap_1()
                            .items_center()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(colors.sidebar_accent))
                            .child(
                                Icon::new(AssetIcon::Database)
                                    .with_size(px(16.))
                                    .text_color(muted),
                            )
                            .child(div().text_size(px(13.)).child(connection_name))
                            .child(
                                Icon::new(IconName::ChevronDown)
                                    .xsmall()
                                    .text_color(muted),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_notice("选择连接未完成", cx)
                            })),
                    )
                    .child(div().text_size(px(12.)).text_color(muted).child("/"))
                    .child(Self::query_crumb(database, colors, cx))
                    .child(div().text_size(px(12.)).text_color(muted).child("/"))
                    .child(Self::query_crumb(schema, colors, cx))
                    .child(div().flex_1())
                    .child(Self::icon_button(
                        "format-sql",
                        Icon::new(AssetIcon::Wand)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, _, cx| this.show_notice("格式化未完成", cx)),
                    ))
                    .child(Self::icon_button(
                        "query-info",
                        Icon::new(AssetIcon::Info)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, _, cx| this.show_notice("查询信息未完成", cx)),
                    ))
                    .child(
                        Button::new("run-query")
                            .primary()
                            .small()
                            .icon(Icon::new(AssetIcon::Play).with_size(px(16.)))
                            .label("运行")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.run_query(window, cx)
                            })),
                    ),
            )
            // editor
            .child(
                v_flex()
                    .h(gpui_kit::relative(0.52))
                    .flex_none()
                    .min_h_0()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(13.))
                    .child(match editor {
                        Some(state) => Editor::new(&state)
                            .appearance(false)
                            .bordered(false)
                            .h(gpui_kit::relative(1.))
                            .into_any_element(),
                        None => div().into_any_element(),
                    }),
            )
            // result header
            .child(
                h_flex()
                    .h(px(44.))
                    .flex_none()
                    .px_3()
                    .gap_1()
                    .items_center()
                    .border_t_1()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        v_flex()
                            .h_full()
                            .justify_center()
                            .border_b_2()
                            .border_color(colors.foreground)
                            .px_1()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("结果"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(muted)
                                            .child(rows.to_string()),
                                    ),
                            ),
                    )
                    .child(div().flex_1())
                    .child(Self::icon_button(
                        "rerun-query",
                        Icon::new(AssetIcon::RotateCw)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| this.run_query(window, cx)),
                    ))
                    .child(Self::icon_button(
                        "download-results",
                        Icon::new(AssetIcon::Download)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, _, cx| this.show_notice("导出未完成", cx)),
                    ))
                    .child(Self::icon_button(
                        "more-results",
                        Icon::new(AssetIcon::Ellipsis)
                            .with_size(px(16.))
                            .text_color(muted)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, _, cx| this.show_notice("更多操作未完成", cx)),
                    )),
            )
            // results
            .child(match self.grid.clone() {
                Some(grid) => div()
                    .flex_1()
                    .min_h_0()
                    .child(self.table_frame(&grid, colors))
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
            })
            // status bar
            .child(
                h_flex()
                    .h(px(32.))
                    .flex_none()
                    .px_3()
                    .gap_3()
                    .items_center()
                    .border_t_1()
                    .border_color(colors.border)
                    .text_size(px(11.))
                    .text_color(muted)
                    .child(format!("{} 行", rows))
                    .child(div().text_color(colors.border).child("|"))
                    .child(div().child("未选择"))
                    .child(div().text_color(colors.border).child("|"))
                    .child(div().child(elapsed))
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

    /// Tabs live in the titlebar row, outside the content card.
    fn tab_bar(&self, leading_toggle: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        h_flex()
            .h(px(44.))
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
                    .child(Self::icon_button(
                        "new-query",
                        Icon::new(AssetIcon::Plus)
                            .with_size(px(16.))
                            .text_color(colors.muted_foreground)
                            .into_any_element(),
                        colors,
                        cx.listener(|this, _, window, cx| this.open_query(window, cx)),
                    ))
                    .child(div().flex_1())
    }

    fn main_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let tab = self.active.and_then(|index| self.tabs.get(index)).cloned();

        let title = if self.show_query {
            "查询".to_string()
        } else if let Some(tab) = &tab {
            format!(
                "{} · {}",
                self.connections
                    .iter()
                    .find(|item| item.id == tab.connection_id)
                    .map(|item| item.name.clone())
                    .unwrap_or_default(),
                tab.database
            )
        } else {
            "选择一张表".to_string()
        };
        let meta = if self.show_query {
            self.query_target()
                .map(|(connection, database, _)| format!("{} · {}", connection.name, database))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let status = if self.loading {
            "加载中…".to_string()
        } else if self.show_query || tab.is_some() {
            format!("{} 行", self.rows.len())
        } else {
            String::new()
        };

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            // context bar for table tabs; the query page has its own toolbar
            .when(!self.show_query, |this| {
                this.child(
                    h_flex()
                        .h(px(44.))
                        .flex_none()
                        .px_4()
                        .gap_3()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .child(div().text_size(px(12.)).text_color(muted).child(meta))
                        .child(div().flex_1())
                        .child(div().text_size(px(12.)).text_color(muted).child(status)),
                )
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
                            .child(self.table_frame(&grid, colors))
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
    }

    /// One tab in the strip: icon, title, optional close button.
    fn tab_chip(
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
        let chip = h_flex()
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
            Some(target) => chip
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
                        this.show_query = false;
                        this.active = Some(index);
                        this.load_active(window, cx);
                    }
                    CloseTarget::Query(index) => {
                        this.show_query = true;
                        this.active_query = Some(index);
                        cx.notify();
                    }
                }))
                .into_any_element(),
            None => chip
                .on_click(cx.listener(|this, _, _, cx| {
                    this.show_query = true;
                    cx.notify();
                }))
                .into_any_element(),
        }
    }
}

/// Divider between the sidebar and the content: never painted. The 9px band
/// still hit-tests and switches to the column-resize cursor, so the shell keeps
/// its line-free look in every state.
fn split_handle() -> ResizeHandleRenderer {
    Rc::new(|_, _, _| Some(div().into_any_element()))
}

impl Omdbx {
    /// One square icon button: both titlebar buttons go through this so their
    /// hit area, radius and hover fill cannot drift apart.
    fn icon_button(
        id: &'static str,
        child: gpui_kit::AnyElement,
        colors: gpui_kit::component::ThemeColor,
        on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        h_flex()
            .id(id)
            .h(px(28.))
            .w(px(28.))
            .flex_none()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .hover(|style| style.bg(colors.secondary_active))
            .child(child)
            .on_click(on_click)
    }

    /// Sidebar glyph: the filled left panel means "expand", the divider means
    /// "collapse", matching the states the button switches between.
    fn sidebar_glyph(open: bool, color: Hsla) -> impl IntoElement {
        let frame = div()
            .w(px(14.))
            .h(px(14.))
            .flex_none()
            .rounded(px(2.))
            .border_1()
            .border_color(color);
        if open {
            frame.child(div().h_full().w(px(1.)).ml(px(3.)).bg(color))
        } else {
            frame.child(div().h_full().w(px(4.)).bg(color))
        }
    }

    fn sidebar_toggle(&self, cx: &mut Context<Self>) -> gpui_kit::Stateful<gpui_kit::Div> {
        let colors = cx.theme().colors;
        Self::icon_button(
            "sidebar-toggle",
            Self::sidebar_glyph(self.sidebar_open, colors.muted_foreground).into_any_element(),
            colors,
            cx.listener(|this, _, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                cx.notify();
            }),
        )
    }

    /// The white content card, inset from the shell by a small gap.
    fn content_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        div()
            .flex_1()
            .h_full()
            .min_w_0()
            .px(px(8.))
            .pb(px(8.))
            // no tab strip: keep the same 8px gap at the top as the sides
            .when(self.tabs.is_empty() && self.queries.is_empty(), |this| {
                this.pt(px(8.))
            })
            .child(
                v_flex()
                    .size_full()
                    .rounded(px(10.))
                    .overflow_hidden()
                    .bg(colors.background)
                    .border_1()
                    .border_color(colors.border)
                    .child(self.main_pane(cx)),
            )
    }
}

impl Render for Omdbx {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let shell: gpui_kit::AnyElement = if self.sidebar_open {
            h_flex()
                .size_full()
                .bg(colors.sidebar)
                .text_color(colors.foreground)
                .child(
                    h_resizable("shell")
                        .with_handle_appearance(split_handle())
                        .child(
                            resizable_panel()
                                .size(px(280.))
                                .size_range(px(180.)..px(420.))
                                .fixed()
                                .child(self.sidebar(cx)),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .h_full()
                                .min_w_0()
                                // no tabs yet: no empty strip and no +, the card
                                // starts at the top of the window
                                .when(!self.tabs.is_empty() || !self.queries.is_empty(), |this| {
                                    this.child(self.tab_bar(false, cx))
                                })
                                .child(self.content_card(cx))
                                .into_any_element(),
                        ),
                )
                .into_any_element()
        } else {
            v_flex()
                .size_full()
                .bg(colors.sidebar)
                .text_color(colors.foreground)
                .child(
                    div()
                        .relative()
                        .h(px(44.))
                        .flex_none()
                        .child(TitleBar::new().h(px(44.)).bg(colors.sidebar))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left(px(0.))
                                .right(px(0.))
                                .h(px(44.))
                                .child(self.tab_bar(true, cx)),
                        ),
                )
                .child(self.content_card(cx))
                .into_any_element()
        };

        div()
            .relative()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &RefreshConnections, _, cx| this.reload(cx)))
            .child(shell)
            .when_some(self.notice.clone(), |this, notice| {
                this.child(
                    div()
                        .absolute()
                        .bottom(px(24.))
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .px(px(16.))
                                .py(px(8.))
                                .rounded_md()
                                .bg(colors.foreground)
                                .text_color(colors.background)
                                .text_size(px(13.))
                                .child(notice),
                        ),
                )
            })
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx: &mut App| {
        gpui_kit::init(cx);
        cx.bind_keys([gpui_kit::KeyBinding::new(
            "cmd-r",
            RefreshConnections,
            None,
        )]);
        Theme::update(cx, |theme| {
            theme.mode = ThemeMode::Light;
            // the shell background; the content card floats on it
            theme.colors.sidebar = gpui_kit::rgb(0xF9F9F9).into();
            // the table draws its own rounded header band and sits on the card
            theme.tokens.table = gpui_kit::transparent_black().into();
            theme.tokens.table_head = gpui_kit::transparent_black().into();
            // flatten the titlebar into the sidebar, like the rest of the shell
            theme.colors.title_bar = theme.colors.sidebar;
            theme.colors.title_bar_border = theme.colors.sidebar;
        });
        gpui_kit::open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::centered(
                    None,
                    gpui_kit::size(px(1100.), px(700.)),
                    cx,
                ))),
                window_min_size: Some(gpui_kit::size(px(900.), px(560.))),
                // transparent titlebar owned by the app, so the sidebar can run
                // to the top edge under the traffic lights
                ..TitleBar::window_options()
            },
            cx,
            |window, cx| {
                // center the traffic lights on the 44px titlebar axis
                #[cfg(target_os = "macos")]
                window.set_traffic_light_position(gpui_kit::point(px(9.), px(15.)));
                cx.new(|cx| Omdbx::new(window, cx))
            },
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
