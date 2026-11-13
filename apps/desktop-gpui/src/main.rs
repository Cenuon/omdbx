//! OMDBX desktop, Rust + GPUI, styled with GPUI Kit.
//! Talks to the existing `dbx-web` sidecar over HTTP so all connection and query
//! logic is shared with the Electron build.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_xlsxwriter::Workbook;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::date_picker::{
    DatePicker, DatePickerEvent, DatePickerState, DateTime,
};
use gpui_kit::component::time_field::{
    TimeField, TimeFieldEvent, TimeFieldState, TimePrecision,
};
use gpui_kit::component::input::{Editor, EditorState, Input, InputEvent, InputState, Enter, IndentInline, MoveDown, MoveUp};
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::Selectable;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable, Theme, ThemeMode, TitleBar,
};
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::base::{h_resizable, resizable_panel, ElementExt, ResizeHandleRenderer};
use gpui_kit::prelude::*;
use gpui_kit::Focusable;
use gpui_kit::{
    deferred, div, px, App, Context, Entity, FocusHandle, FontFeatures, FontWeight, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render, ScrollHandle,
    SharedString, Styled, Window, WindowOptions,
};
use serde::Deserialize;
use serde_json::json;

gpui_kit::actions!(omdbx, [RefreshConnections, ConfirmTab, ConfirmEnter, SuggestUp, SuggestDown]);

#[macro_use]
mod actions;
mod backend;
mod grid;
mod ui;
mod views;

use grid::GridDelegate;
use ui::menu::app_menu;

use backend::API;
const ROW_LIMIT: usize = 1000;

const GRID_ZOOM_LEVELS: &[(&str, f32)] = &[
    ("50%", 0.5),
    ("75%", 0.75),
    ("100%", 1.0),
    ("125%", 1.25),
    ("150%", 1.5),
    ("175%", 1.75),
    ("200%", 2.0),
    ("250%", 2.5),
    ("300%", 3.0),
    ("400%", 4.0),
    ("500%", 5.0),
];

const SUPPORTED_LANGUAGES: [(&str, &str); 7] = [
    ("zh-CN", "简体中文"),
    ("zh-TW", "繁體中文（台灣）"),
    ("zh-HK", "繁體中文（香港）"),
    ("en", "English"),
    ("pt-BR", "Português (Brasil)"),
    ("ru", "Русский"),
    ("it", "Italiano"),
];

fn default_language() -> String {
    "zh-CN".to_string()
}

#[derive(Deserialize)]
struct AppSettings {
    #[serde(default = "default_language")]
    language: String,
}

fn settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/omdbx-gpui/settings.json"))
}

fn load_language() -> String {
    let Some(text) = settings_path().and_then(|path| std::fs::read_to_string(path).ok()) else {
        return default_language();
    };
    let settings: AppSettings = serde_json::from_str(&text).unwrap_or(AppSettings {
        language: default_language(),
    });
    if SUPPORTED_LANGUAGES
        .iter()
        .any(|(code, _)| *code == settings.language)
    {
        settings.language
    } else {
        default_language()
    }
}

fn save_language(language: &str) {
    let Some(path) = settings_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        path,
        serde_json::to_string_pretty(&json!({ "language": language })).unwrap_or_default(),
    );
}

fn date_picker_format(language: &str, with_time: bool) -> String {
    let date = match language {
        "en" => "%m/%d/%Y",
        "pt-BR" | "it" => "%d/%m/%Y",
        "ru" => "%d.%m.%Y",
        _ => "%Y-%m-%d",
    };
    if with_time {
        format!("{date} %H:%M:%S")
    } else {
        date.to_string()
    }
}

/// Upper bound for a whole-table export; the sidecar keeps the rest of the
/// table out of the response.
const EXPORT_MAX_ROWS: usize = 1_000_000;

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

fn backend_error_message(body: &str, status: u16) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        for key in ["detail", "message", "error"] {
            if let Some(message) = value.get(key).and_then(serde_json::Value::as_str) {
                if !message.trim().is_empty() {
                    return message.to_string();
                }
            }
        }
    }
    let body = body.trim();
    if body.is_empty() {
        format!("请求失败（HTTP {status}）")
    } else {
        body.to_string()
    }
}

fn post_json(url: &str, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
    let response = agent()
        .post(url)
        .set("Content-Type", "application/json")
        .send_string(&payload.to_string());
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            return Err(backend_error_message(&body, status));
        }
        Err(error) => return Err(error.to_string()),
    };
    let body = response
        .into_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

fn test_connection_config(config: &serde_json::Value) -> Result<String, String> {
    let value = post_json(
        &format!("{API}/api/connection/test"),
        &json!({ "config": config }),
    )?;
    Ok(value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.get("message").and_then(|message| message.as_str()).map(str::to_string))
        .unwrap_or_else(|| "连接成功".to_string()))
}

fn save_connection_config(config: &serde_json::Value) -> Result<(), String> {
    post_json(
        &format!("{API}/api/connection/save"),
        &json!({ "configs": [config], "removedIds": [] }),
    )?;
    Ok(())
}

#[derive(Clone)]
struct FilterDividerDrag;

impl Render for FilterDividerDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct PropsDividerDrag;

impl Render for PropsDividerDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct QueryDividerDrag;

impl Render for QueryDividerDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Connection {
    id: String,
    name: String,
    db_type: String,
    #[serde(default)]
    driver_profile: Option<String>,
    #[serde(default)]
    default_schema: Option<String>,
}

impl Connection {
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
    #[serde(default)]
    is_primary_key: bool,
    #[serde(default)]
    is_nullable: bool,
    #[serde(default)]
    column_default: Option<String>,
    #[serde(default)]
    extra: Option<String>,
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

fn fetch_table_props(
    connection_id: &str,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<TableProps, String> {
    let query = format!(
        "connection_id={}&database={}&schema={}&table={}",
        encode(connection_id),
        encode(database),
        encode(schema),
        encode(table)
    );
    let ddl = get_json(&format!("{API}/api/schema/ddl?{query}"))?;
    let columns = fetch_columns(connection_id, database, schema, table).unwrap_or_default();
    let indexes = get_json(&format!("{API}/api/schema/indexes?{query}"))?;
    let foreign_keys = get_json(&format!("{API}/api/schema/foreign-keys?{query}"))?;
    let triggers = get_json(&format!("{API}/api/schema/triggers?{query}"))?;
    Ok(TableProps {
        ddl: ddl.as_str().unwrap_or_default().to_string(),
        columns,
        indexes: serde_json::from_value(indexes).unwrap_or_default(),
        foreign_keys: serde_json::from_value(foreign_keys).unwrap_or_default(),
        triggers: serde_json::from_value(triggers).unwrap_or_default(),
    })
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
    has_more: bool,
}

fn fetch_rows(
    connection: &Connection,
    database: &str,
    schema: &str,
    table: &str,
    where_clause: &str,
    order_clause: &str,
    row_offset: usize,
    execution_id: Option<&str>,
) -> Result<TableData, String> {
    let sql = select_sql(connection, table, where_clause, order_clause);
    let fetch = ROW_LIMIT + 1;
    let db_type = connection.db_type.to_ascii_lowercase();
    let sql = match db_type.as_str() {
        "sqlserver" => {
            let order = if order_clause.is_empty() {
                " ORDER BY (SELECT NULL)".to_string()
            } else {
                String::new()
            };
            format!(
                "{sql}{order} OFFSET {row_offset} ROWS FETCH NEXT {fetch} ROWS ONLY"
            )
        }
        "oracle" => {
            let order = if order_clause.is_empty() {
                " ORDER BY 1".to_string()
            } else {
                String::new()
            };
            format!(
                "{sql}{order} OFFSET {row_offset} ROWS FETCH NEXT {fetch} ROWS ONLY"
            )
        }
        _ => format!("{sql} LIMIT {fetch} OFFSET {row_offset}"),
    };
    let mut data = run_sql(connection, database, schema, &sql, fetch, execution_id)?;
    let has_more = data.rows.len() > ROW_LIMIT;
    data.rows.truncate(ROW_LIMIT);
    data.has_more = has_more;
    Ok(data)
}

fn select_sql(
    connection: &Connection,
    table: &str,
    where_clause: &str,
    order_clause: &str,
) -> String {
    let mut sql = format!("SELECT * FROM {}", connection.quote(table));
    if !where_clause.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(where_clause);
    }
    if !order_clause.is_empty() {
        sql.push_str(" ORDER BY ");
        sql.push_str(order_clause);
    }
    sql
}

/// Whole-table export: one query without paging so every row is consistent
/// with the filter and order currently in the toolbar.
fn fetch_all_rows(
    connection: &Connection,
    database: &str,
    schema: &str,
    table: &str,
    where_clause: &str,
    order_clause: &str,
) -> Result<TableData, String> {
    let sql = select_sql(connection, table, where_clause, order_clause);
    let mut data = run_sql(connection, database, schema, &sql, EXPORT_MAX_ROWS + 1, None)?;
    if data.rows.len() > EXPORT_MAX_ROWS {
        data.rows.truncate(EXPORT_MAX_ROWS);
    }
    Ok(data)
}

fn explain_sql(connection: &Connection, sql: &str) -> String {
    let sql = sql.trim().trim_end_matches(';');
    if sql.to_ascii_uppercase().starts_with("EXPLAIN") {
        return sql.to_string();
    }
    let prefix = match connection.db_type.to_ascii_lowercase().as_str() {
        "sqlite" | "turso" | "rqlite" | "cloudflare-d1" => "EXPLAIN QUERY PLAN ",
        _ => "EXPLAIN ",
    };
    format!("{prefix}{sql}")
}

fn run_sql(
    connection: &Connection,
    database: &str,
    schema: &str,
    sql: &str,
    max_rows: usize,
    execution_id: Option<&str>,
) -> Result<TableData, String> {
    let mut payload = json!({
        "connectionId": connection.id,
        "database": database,
        "schema": schema,
        "sql": sql,
        "maxRows": max_rows,
    });
    if let Some(execution_id) = execution_id {
        payload["executionId"] = json!(execution_id);
    }
    let value = post_json(&format!("{API}/api/query/execute"), &payload)?;
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
    let has_more = value
        .get("has_more")
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false);
    if let Some(error) = value.get("error").and_then(|error| error.as_object()) {
        let message = error
            .get("detail")
            .or_else(|| error.get("message"))
            .and_then(|message| message.as_str())
            .unwrap_or("查询失败");
        return Err(message.to_string());
    }
    Ok(TableData {
        columns,
        column_types,
        rows,
        has_more,
    })
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

fn last_word(value: &str) -> String {
    value
        .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .unwrap_or_default()
        .to_string()
}

fn format_sql_text(sql: &str, db_type: &str) -> String {
    let dialect = match db_type {
        "postgres" | "postgresql" => sqlformat::Dialect::PostgreSql,
        "sqlserver" | "mssql" => sqlformat::Dialect::SQLServer,
        _ => sqlformat::Dialect::Generic,
    };
    let options = sqlformat::FormatOptions {
        uppercase: Some(true),
        indent: sqlformat::Indent::Spaces(2),
        max_inline_top_level: Some(80),
        max_inline_arguments: Some(60),
        dialect,
        ..Default::default()
    };
    sqlformat::format(sql, &sqlformat::QueryParams::None, &options)
}

fn sql_spans(value: &str) -> Vec<gpui_kit::AnyElement> {
    let mut segments: Vec<(String, bool)> = Vec::new();
    let mut buffer = String::new();
    let mut is_word = false;
    for ch in value.chars() {
        let next_is_word = ch.is_alphanumeric() || ch == '_';
        if !buffer.is_empty() && next_is_word != is_word {
            segments.push((buffer, is_word));
            buffer = String::new();
        }
        buffer.push(ch);
        is_word = next_is_word;
    }
    if !buffer.is_empty() {
        segments.push((buffer, is_word));
    }
    segments
        .into_iter()
        .map(|(text, word)| {
            let keyword = word
                && SQL_KEYWORDS
                    .iter()
                    .any(|keyword| keyword.eq_ignore_ascii_case(&text));
            div()
                .when(keyword, |this| {
                    this.text_color(gpui_kit::rgb(0x2F6BFF))
                        .font_weight(FontWeight::SEMIBOLD)
                })
                .child(text)
                .into_any_element()
        })
        .collect()
}

fn cell_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn export_csv(data: &TableData) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str(
        &data
            .columns
            .iter()
            .map(|value| csv_field(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    for row in &data.rows {
        out.push_str(
            &row.iter()
                .map(|value| csv_field(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

fn export_json(data: &TableData) -> Result<String, String> {
    let rows: Vec<serde_json::Value> = data
        .rows
        .iter()
        .map(|row| {
            let mut object = serde_json::Map::new();
            for (index, column) in data.columns.iter().enumerate() {
                object.insert(
                    column.clone(),
                    json!(row.get(index).cloned().unwrap_or_default()),
                );
            }
            serde_json::Value::Object(object)
        })
        .collect();
    serde_json::to_string_pretty(&rows).map_err(|error| error.to_string())
}

fn export_markdown(data: &TableData) -> String {
    let escape = |value: &str| value.replace('|', "\\|").replace('\n', "<br>");
    let mut out = String::new();
    out.push('|');
    for column in &data.columns {
        out.push(' ');
        out.push_str(&escape(column));
        out.push_str(" |");
    }
    out.push('\n');
    out.push('|');
    for _ in &data.columns {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in &data.rows {
        out.push('|');
        for value in row {
            out.push(' ');
            out.push_str(&escape(value));
            out.push_str(" |");
        }
        out.push('\n');
    }
    out
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn export_html(data: &TableData) -> String {
    let mut out = String::from(
        "<!doctype html>\n<meta charset=\"utf-8\">\n<table border=\"1\" cellspacing=\"0\" cellpadding=\"6\">\n<thead><tr>",
    );
    for column in &data.columns {
        out.push_str("<th>");
        out.push_str(&html_escape(column));
        out.push_str("</th>");
    }
    out.push_str("</tr></thead>\n<tbody>\n");
    for row in &data.rows {
        out.push_str("<tr>");
        for value in row {
            out.push_str("<td>");
            out.push_str(&html_escape(value));
            out.push_str("</td>");
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
    out
}

fn sql_literal(value: &str, data_type: &str) -> String {
    if value == "NULL" {
        return "NULL".to_string();
    }
    if is_numeric_type(data_type) && value.parse::<f64>().is_ok() {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn edit_sql_literal(value: &str, data_type: &str, db_type: &str) -> String {
    if value == "NULL" {
        return "NULL".to_string();
    }
    if is_numeric_type(data_type) && value.parse::<f64>().is_ok() {
        return value.to_string();
    }
    let mysql = matches!(
        db_type,
        "mysql" | "mariadb" | "tidb" | "oceanbase" | "goldendb" | "databend"
    );
    let escaped = if mysql {
        value.replace('\\', "\\\\").replace('\'', "''")
    } else {
        value.replace('\'', "''")
    };
    format!("'{escaped}'")
}

fn update_row_sql(
    connection: &Connection,
    table: &str,
    assignments: &[(String, String, String)],
    keys: &[(String, String, String)],
) -> Result<String, String> {
    if assignments.is_empty() {
        return Err("没有需要保存的修改".to_string());
    }
    if keys.is_empty() {
        return Err("这张表没有主键，暂不支持编辑".to_string());
    }
    let assignments = assignments
        .iter()
        .map(|(column, column_type, value)| {
            format!(
                "{} = {}",
                connection.quote(column),
                edit_sql_literal(value, column_type, &connection.db_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let predicates = keys
        .iter()
        .map(|(name, data_type, original)| {
            if original == "NULL" {
                format!("{} IS NULL", connection.quote(name))
            } else {
                format!(
                    "{} = {}",
                    connection.quote(name),
                    edit_sql_literal(original, data_type, &connection.db_type)
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    Ok(format!(
        "UPDATE {} SET {assignments} WHERE {predicates}",
        connection.quote(table)
    ))
}

fn delete_row_sql(
    connection: &Connection,
    table: &str,
    keys: &[(String, String, String)],
) -> Result<String, String> {
    if keys.is_empty() {
        return Err("这张表没有主键，无法删除".to_string());
    }
    let predicates = keys
        .iter()
        .map(|(name, data_type, original)| {
            if original == "NULL" {
                format!("{} IS NULL", connection.quote(name))
            } else {
                format!(
                    "{} = {}",
                    connection.quote(name),
                    edit_sql_literal(original, data_type, &connection.db_type)
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    Ok(format!(
        "DELETE FROM {} WHERE {predicates}",
        connection.quote(table)
    ))
}

fn insert_row_sql(
    connection: &Connection,
    table: &str,
    columns: &[String],
    meta: &[ColumnMeta],
    values: &[String],
) -> Result<String, String> {
    let mut names = Vec::new();
    let mut literals = Vec::new();
    for (index, value) in values.iter().enumerate() {
        if value.is_empty() {
            continue;
        }
        let Some(column) = columns.get(index) else {
            continue;
        };
        let data_type = meta
            .get(index)
            .map(|meta| meta.data_type.as_str())
            .unwrap_or_default();
        names.push(connection.quote(column));
        literals.push(edit_sql_literal(value, data_type, &connection.db_type));
    }
    if names.is_empty() {
        let db_type = connection.db_type.to_ascii_lowercase();
        if matches!(
            db_type.as_str(),
            "mysql" | "mariadb" | "tidb" | "oceanbase" | "goldendb" | "databend"
        ) {
            Ok(format!(
                "INSERT INTO {} () VALUES ()",
                connection.quote(table)
            ))
        } else if db_type == "oracle" {
            Err("Oracle 新增行需要至少填写一个字段".to_string())
        } else {
            Ok(format!(
                "INSERT INTO {} DEFAULT VALUES",
                connection.quote(table)
            ))
        }
    } else {
        Ok(format!(
            "INSERT INTO {} ({}) VALUES ({})",
            connection.quote(table),
            names.join(", "),
            literals.join(", ")
        ))
    }
}

fn export_sql_insert(data: &TableData, table: &str, connection: &Connection) -> String {
    let columns = data
        .columns
        .iter()
        .map(|column| connection.quote(column))
        .collect::<Vec<_>>()
        .join(", ");
    let table = connection.quote(table);
    let mut out = String::new();
    for row in &data.rows {
        let values = row
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let data_type = data
                    .column_types
                    .get(index)
                    .map(String::as_str)
                    .unwrap_or_default();
                sql_literal(value, data_type)
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("INSERT INTO {table} ({columns}) VALUES ({values});\n"));
    }
    out
}

fn export_txt(data: &TableData) -> String {
    let clean = |value: &str| value.replace(['\t', '\n', '\r'], " ");
    let mut out = String::new();
    out.push_str(
        &data
            .columns
            .iter()
            .map(|value| clean(value))
            .collect::<Vec<_>>()
            .join("\t"),
    );
    out.push('\n');
    for row in &data.rows {
        out.push_str(
            &row.iter()
                .map(|value| clean(value))
                .collect::<Vec<_>>()
                .join("\t"),
        );
        out.push('\n');
    }
    out
}

fn write_xlsx(path: &Path, data: &TableData, table: &str) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    let name: String = table
        .chars()
        .map(|ch| match ch {
            '[' | ']' | ':' | '*' | '?' | '/' | '\\' => '_',
            other => other,
        })
        .take(31)
        .collect();
    let _ = sheet.set_name(if name.is_empty() {
        "Sheet1".to_string()
    } else {
        name
    });
    for (index, column) in data.columns.iter().enumerate() {
        sheet
            .write_string(0, index as u16, column)
            .map_err(|error| error.to_string())?;
    }
    for (row_index, row) in data.rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            sheet
                .write_string((row_index + 1) as u32, column_index as u16, value)
                .map_err(|error| error.to_string())?;
        }
    }
    workbook.save(path).map_err(|error| error.to_string())
}

fn write_export(
    path: &Path,
    format: ExportFormat,
    data: &TableData,
    table: &str,
    connection: &Connection,
) -> Result<(), String> {
    if format == ExportFormat::Xlsx {
        return write_xlsx(path, data, table);
    }
    let text = match format {
        ExportFormat::Csv => export_csv(data),
        ExportFormat::Json => export_json(data)?,
        ExportFormat::Markdown => export_markdown(data),
        ExportFormat::Html => export_html(data),
        ExportFormat::SqlInsert => export_sql_insert(data, table, connection),
        ExportFormat::Txt => export_txt(data),
        ExportFormat::Xlsx => unreachable!(),
    };
    std::fs::write(path, text).map_err(|error| error.to_string())
}

fn export_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Downloads"))
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn safe_export_stem(value: &str) -> String {
    let stem: String = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let stem = stem.trim_matches(|ch| ch == ' ' || ch == '.').to_string();
    if stem.is_empty() {
        "export".to_string()
    } else {
        stem
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
#[derive(Clone)]
enum CellEditor {
    Text(Entity<InputState>),
    Date(Entity<DatePickerState>),
    Time(Entity<TimeFieldState>),
}

#[derive(Clone)]
struct EditingCell {
    /// Row index in the loaded page, and data-column index (without `#`).
    row: usize,
    col: usize,
    editor: CellEditor,
}

#[derive(Clone)]
struct PendingEdit {
    tab_key: String,
    page: usize,
    column: String,
    column_type: String,
    original: String,
    value: String,
    /// Primary-key columns and the values that identify this row.
    keys: Vec<(String, String, String)>,
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

fn is_temporal_type(data_type: &str) -> bool {
    let ty = data_type.to_ascii_lowercase();
    ["date", "time", "year"].iter().any(|needle| ty.contains(needle))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemporalKind {
    Text,
    Date,
    DateTime,
    Time,
}

fn temporal_kind(data_type: &str) -> TemporalKind {
    let ty = data_type.to_ascii_lowercase();
    if ty.contains("timestamp") || ty.contains("datetime") {
        TemporalKind::DateTime
    } else if ty.contains("date") {
        TemporalKind::Date
    } else if ty.contains("time") {
        TemporalKind::Time
    } else {
        TemporalKind::Text
    }
}

fn parse_datetime(value: &str) -> Option<NaiveDateTime> {
    let value = value.trim();
    if value.is_empty() || value == "NULL" {
        return None;
    }
    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%dT%H:%M:%S",
    ] {
        if let Ok(value) = NaiveDateTime::parse_from_str(value, format) {
            return Some(value);
        }
    }
    for format in ["%Y-%m-%d", "%Y/%m/%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(value, format) {
            return date.and_hms_opt(0, 0, 0);
        }
    }
    None
}

fn parse_time(value: &str) -> Option<NaiveTime> {
    let value = value.trim();
    if value.is_empty() || value == "NULL" {
        return None;
    }
    for format in ["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"] {
        if let Ok(value) = NaiveTime::parse_from_str(value, format) {
            return Some(value);
        }
    }
    None
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
    connection_id: String,
    database: String,
    schema: String,
}

#[derive(Clone)]
struct QueryEditTarget {
    query_key: String,
    connection: Connection,
    database: String,
    schema: String,
    table: String,
}

struct EditTarget {
    tab_key: String,
    page: usize,
    connection: Connection,
    database: String,
    schema: String,
    table: String,
    is_query: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResultTab {
    Results,
    Summary,
    Info,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QueryStatus {
    Running,
    Success,
    Failed,
    Cancelled,
}

struct QueryInfo {
    sql: String,
    connection: String,
    database: String,
    schema: String,
    elapsed_ms: Option<u128>,
    rows: usize,
    columns: usize,
    status: QueryStatus,
    error: Option<String>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropsTab {
    Ddl,
    Fields,
    Indexes,
    ForeignKeys,
    Triggers,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct IndexInfo {
    #[serde(default)]
    name: String,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    is_unique: bool,
    #[serde(default)]
    is_primary: bool,
    #[serde(default)]
    index_type: Option<String>,
    #[serde(default)]
    included_columns: Option<Vec<String>>,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ForeignKeyInfo {
    #[serde(default)]
    name: String,
    #[serde(default)]
    column: String,
    #[serde(default)]
    ref_schema: Option<String>,
    #[serde(default)]
    ref_table: String,
    #[serde(default)]
    ref_column: String,
    #[serde(default)]
    on_update: Option<String>,
    #[serde(default)]
    on_delete: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TriggerInfo {
    #[serde(default)]
    name: String,
    #[serde(default)]
    event: String,
    #[serde(default)]
    timing: String,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    statement: Option<String>,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Default)]
struct TableProps {
    ddl: String,
    columns: Vec<ColumnMeta>,
    indexes: Vec<IndexInfo>,
    foreign_keys: Vec<ForeignKeyInfo>,
    triggers: Vec<TriggerInfo>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SuggestField {
    Where,
    Order,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Csv,
    Xlsx,
    Json,
    Markdown,
    Html,
    SqlInsert,
    Txt,
}

impl ExportFormat {
    const ALL: [ExportFormat; 7] = [
        ExportFormat::Csv,
        ExportFormat::Xlsx,
        ExportFormat::Json,
        ExportFormat::Markdown,
        ExportFormat::Html,
        ExportFormat::SqlInsert,
        ExportFormat::Txt,
    ];

    fn label(self) -> &'static str {
        match self {
            ExportFormat::Csv => "CSV",
            ExportFormat::Xlsx => "XLSX",
            ExportFormat::Json => "JSON",
            ExportFormat::Markdown => "Markdown",
            ExportFormat::Html => "HTML",
            ExportFormat::SqlInsert => "SQL INSERT",
            ExportFormat::Txt => "TXT",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "md",
            ExportFormat::Html => "html",
            ExportFormat::SqlInsert => "sql",
            ExportFormat::Txt => "txt",
        }
    }
}

struct Suggestion {
    text: String,
    detail: String,
}

const SQL_KEYWORDS: [&str; 20] = [
    "AND", "OR", "NOT", "NULL", "LIKE", "IN", "IS", "BETWEEN", "EXISTS", "CASE", "WHEN",
    "THEN", "ELSE", "END", "ASC", "DESC", "TRUE", "FALSE", "REGEXP", "DISTINCT",
];

/// What each flat tree row represents, used to pin ancestors while scrolling.
enum TreeEntry {
    Connection(String),
    Database(String, String),
    Table(String, String),
}

#[derive(Clone, Copy)]
enum WidthTarget {
    Column(usize),
    All,
    RowHeight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DatabaseCategory {
    Sql,
    Analytics,
    Domestic,
    Lightweight,
    Document,
    GraphAi,
    Timeseries,
    MessageQueue,
    RegistryConfig,
}

impl DatabaseCategory {
    fn label(self) -> &'static str {
        match self {
            Self::Sql => "关系型数据库",
            Self::Analytics => "数仓分析",
            Self::Domestic => "国产数据库",
            Self::Lightweight => "轻量数据库",
            Self::Document => "文档/缓存/检索",
            Self::GraphAi => "图谱/向量",
            Self::Timeseries => "时序数据库",
            Self::MessageQueue => "消息队列",
            Self::RegistryConfig => "注册与配置",
        }
    }
}

const DATABASE_CATEGORIES: [DatabaseCategory; 9] = [
    DatabaseCategory::Sql,
    DatabaseCategory::Analytics,
    DatabaseCategory::Domestic,
    DatabaseCategory::Lightweight,
    DatabaseCategory::Document,
    DatabaseCategory::GraphAi,
    DatabaseCategory::Timeseries,
    DatabaseCategory::MessageQueue,
    DatabaseCategory::RegistryConfig,
];

#[derive(Clone, Copy)]
struct DatabaseTypeSpec {
    key: &'static str,
    label: &'static str,
    category: DatabaseCategory,
    db_type: &'static str,
    default_port: u16,
    default_user: &'static str,
    local_file: bool,
    icon: &'static str,
    driver_profile: Option<&'static str>,
}

const DATABASE_TYPES: &[DatabaseTypeSpec] = &[
    DatabaseTypeSpec { key: "mysql", label: "MySQL", category: DatabaseCategory::Sql, db_type: "mysql", default_port: 3306, default_user: "root", local_file: false, icon: "mysql", driver_profile: None },
    DatabaseTypeSpec { key: "postgres", label: "PostgreSQL", category: DatabaseCategory::Sql, db_type: "postgres", default_port: 5432, default_user: "postgres", local_file: false, icon: "postgres", driver_profile: None },
    DatabaseTypeSpec { key: "sqlserver", label: "SQL Server", category: DatabaseCategory::Sql, db_type: "sqlserver", default_port: 1433, default_user: "sa", local_file: false, icon: "sqlserver", driver_profile: None },
    DatabaseTypeSpec { key: "oracle", label: "Oracle", category: DatabaseCategory::Sql, db_type: "oracle", default_port: 1521, default_user: "system", local_file: false, icon: "oracle", driver_profile: None },
    DatabaseTypeSpec { key: "mariadb", label: "MariaDB", category: DatabaseCategory::Sql, db_type: "mysql", default_port: 3306, default_user: "root", local_file: false, icon: "mariadb", driver_profile: Some("mariadb") },
    DatabaseTypeSpec { key: "firebird", label: "Firebird", category: DatabaseCategory::Sql, db_type: "firebird", default_port: 3050, default_user: "SYSDBA", local_file: false, icon: "firebird", driver_profile: None },
    DatabaseTypeSpec { key: "cockroachdb", label: "CockroachDB", category: DatabaseCategory::Sql, db_type: "postgres", default_port: 26257, default_user: "root", local_file: false, icon: "cockroachdb", driver_profile: Some("cockroachdb") },
    DatabaseTypeSpec { key: "db2", label: "DB2", category: DatabaseCategory::Sql, db_type: "db2", default_port: 50000, default_user: "db2inst1", local_file: false, icon: "db2", driver_profile: None },
    DatabaseTypeSpec { key: "informix", label: "Informix", category: DatabaseCategory::Sql, db_type: "informix", default_port: 9088, default_user: "informix", local_file: false, icon: "informix", driver_profile: None },
    DatabaseTypeSpec { key: "jdbcx", label: "JDBCX", category: DatabaseCategory::Sql, db_type: "jdbc", default_port: 0, default_user: "", local_file: true, icon: "jdbcx", driver_profile: None },
    DatabaseTypeSpec { key: "spanner", label: "Cloud Spanner", category: DatabaseCategory::Sql, db_type: "spanner", default_port: 443, default_user: "", local_file: false, icon: "spanner", driver_profile: None },
    DatabaseTypeSpec { key: "iris", label: "IRIS", category: DatabaseCategory::Sql, db_type: "iris", default_port: 1972, default_user: "_SYSTEM", local_file: false, icon: "iris", driver_profile: None },
    DatabaseTypeSpec { key: "cache", label: "InterSystems Caché", category: DatabaseCategory::Sql, db_type: "iris", default_port: 1972, default_user: "_SYSTEM", local_file: false, icon: "iris", driver_profile: None },
    DatabaseTypeSpec { key: "custom_mysql", label: "Custom (MySQL)", category: DatabaseCategory::Sql, db_type: "mysql", default_port: 3306, default_user: "root", local_file: false, icon: "mysql", driver_profile: None },
    DatabaseTypeSpec { key: "custom_postgres", label: "Custom (PostgreSQL)", category: DatabaseCategory::Sql, db_type: "postgres", default_port: 5432, default_user: "postgres", local_file: false, icon: "postgres", driver_profile: None },
    DatabaseTypeSpec { key: "dolt", label: "Dolt", category: DatabaseCategory::Sql, db_type: "mysql", default_port: 3306, default_user: "root", local_file: false, icon: "dolt", driver_profile: Some("dolt") },
    DatabaseTypeSpec { key: "clickhouse", label: "ClickHouse", category: DatabaseCategory::Analytics, db_type: "clickhouse", default_port: 8123, default_user: "default", local_file: false, icon: "clickhouse", driver_profile: None },
    DatabaseTypeSpec { key: "databend", label: "Databend", category: DatabaseCategory::Analytics, db_type: "databend", default_port: 8000, default_user: "databend", local_file: false, icon: "databend", driver_profile: None },
    DatabaseTypeSpec { key: "starrocks", label: "StarRocks", category: DatabaseCategory::Analytics, db_type: "mysql", default_port: 9030, default_user: "root", local_file: false, icon: "starrocks", driver_profile: Some("starrocks") },
    DatabaseTypeSpec { key: "redshift", label: "Amazon Redshift", category: DatabaseCategory::Analytics, db_type: "redshift", default_port: 5439, default_user: "awsuser", local_file: false, icon: "redshift", driver_profile: None },
    DatabaseTypeSpec { key: "snowflake", label: "Snowflake", category: DatabaseCategory::Analytics, db_type: "snowflake", default_port: 443, default_user: "", local_file: false, icon: "snowflake", driver_profile: None },
    DatabaseTypeSpec { key: "trino", label: "Trino", category: DatabaseCategory::Analytics, db_type: "trino", default_port: 8080, default_user: "", local_file: false, icon: "trino", driver_profile: None },
    DatabaseTypeSpec { key: "bigquery", label: "BigQuery", category: DatabaseCategory::Analytics, db_type: "bigquery", default_port: 443, default_user: "", local_file: false, icon: "bigquery", driver_profile: None },
    DatabaseTypeSpec { key: "databricks", label: "Databricks SQL", category: DatabaseCategory::Analytics, db_type: "databricks", default_port: 443, default_user: "token", local_file: false, icon: "databricks", driver_profile: None },
    DatabaseTypeSpec { key: "tidb", label: "TiDB", category: DatabaseCategory::Domestic, db_type: "mysql", default_port: 4000, default_user: "root", local_file: false, icon: "tidb", driver_profile: Some("tidb") },
    DatabaseTypeSpec { key: "oceanbase", label: "OceanBase", category: DatabaseCategory::Domestic, db_type: "mysql", default_port: 2883, default_user: "root", local_file: false, icon: "oceanbase", driver_profile: Some("oceanbase") },
    DatabaseTypeSpec { key: "dm", label: "达梦 Dameng", category: DatabaseCategory::Domestic, db_type: "dameng", default_port: 5236, default_user: "SYSDBA", local_file: false, icon: "dm", driver_profile: None },
    DatabaseTypeSpec { key: "kingbase", label: "金仓 KingbaseES", category: DatabaseCategory::Domestic, db_type: "kingbase", default_port: 54321, default_user: "system", local_file: false, icon: "kingbase", driver_profile: None },
    DatabaseTypeSpec { key: "opengauss", label: "openGauss", category: DatabaseCategory::Domestic, db_type: "opengauss", default_port: 5432, default_user: "gaussdb", local_file: false, icon: "opengauss", driver_profile: None },
    DatabaseTypeSpec { key: "gbase", label: "南大通用 GBase", category: DatabaseCategory::Domestic, db_type: "gbase", default_port: 5258, default_user: "gbasedbt", local_file: false, icon: "gbase", driver_profile: None },
    DatabaseTypeSpec { key: "highgo", label: "瀚高 HighGo", category: DatabaseCategory::Domestic, db_type: "highgo", default_port: 5866, default_user: "highgo", local_file: false, icon: "highgo", driver_profile: None },
    DatabaseTypeSpec { key: "polardb", label: "PolarDB", category: DatabaseCategory::Domestic, db_type: "mysql", default_port: 3306, default_user: "root", local_file: false, icon: "polardb", driver_profile: Some("polardb") },
    DatabaseTypeSpec { key: "sqlite", label: "SQLite", category: DatabaseCategory::Lightweight, db_type: "sqlite", default_port: 0, default_user: "", local_file: true, icon: "sqlite", driver_profile: None },
    DatabaseTypeSpec { key: "duckdb", label: "DuckDB", category: DatabaseCategory::Lightweight, db_type: "duckdb", default_port: 0, default_user: "", local_file: true, icon: "duckdb", driver_profile: None },
    DatabaseTypeSpec { key: "h2", label: "H2", category: DatabaseCategory::Lightweight, db_type: "h2", default_port: 0, default_user: "sa", local_file: true, icon: "h2", driver_profile: None },
    DatabaseTypeSpec { key: "turso", label: "Turso", category: DatabaseCategory::Lightweight, db_type: "turso", default_port: 443, default_user: "", local_file: false, icon: "turso", driver_profile: None },
    DatabaseTypeSpec { key: "rqlite", label: "RQLite", category: DatabaseCategory::Lightweight, db_type: "rqlite", default_port: 4001, default_user: "", local_file: false, icon: "rqlite", driver_profile: None },
    DatabaseTypeSpec { key: "cloudflare-d1", label: "Cloudflare D1", category: DatabaseCategory::Lightweight, db_type: "cloudflare-d1", default_port: 443, default_user: "", local_file: false, icon: "cloudflare-d1", driver_profile: None },
    DatabaseTypeSpec { key: "mongodb", label: "MongoDB", category: DatabaseCategory::Document, db_type: "mongodb", default_port: 27017, default_user: "", local_file: false, icon: "mongodb", driver_profile: None },
    DatabaseTypeSpec { key: "redis", label: "Redis", category: DatabaseCategory::Document, db_type: "redis", default_port: 6379, default_user: "", local_file: false, icon: "redis", driver_profile: None },
    DatabaseTypeSpec { key: "elasticsearch", label: "Elasticsearch", category: DatabaseCategory::Document, db_type: "elasticsearch", default_port: 9200, default_user: "", local_file: false, icon: "elasticsearch", driver_profile: None },
    DatabaseTypeSpec { key: "easysearch", label: "Easysearch", category: DatabaseCategory::Document, db_type: "easysearch", default_port: 9200, default_user: "", local_file: false, icon: "easysearch", driver_profile: None },
    DatabaseTypeSpec { key: "solr", label: "Apache Solr", category: DatabaseCategory::Document, db_type: "solr", default_port: 8983, default_user: "", local_file: false, icon: "solr", driver_profile: None },
    DatabaseTypeSpec { key: "couchdb", label: "Apache CouchDB", category: DatabaseCategory::Document, db_type: "couchdb", default_port: 5984, default_user: "", local_file: false, icon: "couchdb", driver_profile: None },
    DatabaseTypeSpec { key: "cassandra", label: "Cassandra", category: DatabaseCategory::Document, db_type: "cassandra", default_port: 9042, default_user: "cassandra", local_file: false, icon: "cassandra", driver_profile: None },
    DatabaseTypeSpec { key: "neo4j", label: "Neo4j", category: DatabaseCategory::GraphAi, db_type: "neo4j", default_port: 7687, default_user: "neo4j", local_file: false, icon: "neo4j", driver_profile: None },
    DatabaseTypeSpec { key: "nebula", label: "NebulaGraph", category: DatabaseCategory::GraphAi, db_type: "nebula", default_port: 9669, default_user: "root", local_file: false, icon: "nebula", driver_profile: None },
    DatabaseTypeSpec { key: "qdrant", label: "Qdrant", category: DatabaseCategory::GraphAi, db_type: "qdrant", default_port: 6333, default_user: "", local_file: false, icon: "qdrant", driver_profile: None },
    DatabaseTypeSpec { key: "milvus", label: "Milvus", category: DatabaseCategory::GraphAi, db_type: "milvus", default_port: 19530, default_user: "root", local_file: false, icon: "milvus", driver_profile: None },
    DatabaseTypeSpec { key: "weaviate", label: "Weaviate", category: DatabaseCategory::GraphAi, db_type: "weaviate", default_port: 8080, default_user: "", local_file: false, icon: "weaviate", driver_profile: None },
    DatabaseTypeSpec { key: "chromadb", label: "ChromaDB", category: DatabaseCategory::GraphAi, db_type: "chromadb", default_port: 8000, default_user: "", local_file: false, icon: "chromadb", driver_profile: None },
    DatabaseTypeSpec { key: "influxdb", label: "InfluxDB", category: DatabaseCategory::Timeseries, db_type: "influxdb", default_port: 8086, default_user: "", local_file: false, icon: "influxdb", driver_profile: None },
    DatabaseTypeSpec { key: "tdengine", label: "TDengine", category: DatabaseCategory::Timeseries, db_type: "tdengine", default_port: 6041, default_user: "root", local_file: false, icon: "tdengine", driver_profile: None },
    DatabaseTypeSpec { key: "questdb", label: "QuestDB", category: DatabaseCategory::Timeseries, db_type: "questdb", default_port: 8812, default_user: "questdb", local_file: false, icon: "questdb", driver_profile: None },
    DatabaseTypeSpec { key: "iotdb", label: "Apache IoTDB", category: DatabaseCategory::Timeseries, db_type: "iotdb", default_port: 6667, default_user: "root", local_file: false, icon: "iotdb", driver_profile: None },
    DatabaseTypeSpec { key: "victoriametrics", label: "VictoriaMetrics", category: DatabaseCategory::Timeseries, db_type: "victoriametrics", default_port: 8428, default_user: "", local_file: false, icon: "victoriametrics", driver_profile: None },
    DatabaseTypeSpec { key: "kafka", label: "Apache Kafka", category: DatabaseCategory::MessageQueue, db_type: "mq", default_port: 9092, default_user: "", local_file: false, icon: "kafka", driver_profile: Some("kafka") },
    DatabaseTypeSpec { key: "rocketmq", label: "Apache RocketMQ", category: DatabaseCategory::MessageQueue, db_type: "mq", default_port: 9876, default_user: "", local_file: false, icon: "rocketmq", driver_profile: Some("rocketmq") },
    DatabaseTypeSpec { key: "rabbitmq", label: "RabbitMQ", category: DatabaseCategory::MessageQueue, db_type: "mq", default_port: 5672, default_user: "", local_file: false, icon: "rabbitmq", driver_profile: Some("rabbitmq") },
    DatabaseTypeSpec { key: "mqtt", label: "MQTT", category: DatabaseCategory::MessageQueue, db_type: "mqtt", default_port: 1883, default_user: "", local_file: false, icon: "mqtt", driver_profile: None },
    DatabaseTypeSpec { key: "etcd", label: "etcd", category: DatabaseCategory::RegistryConfig, db_type: "etcd", default_port: 2379, default_user: "", local_file: false, icon: "etcd", driver_profile: None },
    DatabaseTypeSpec { key: "zookeeper", label: "Apache ZooKeeper", category: DatabaseCategory::RegistryConfig, db_type: "zookeeper", default_port: 2181, default_user: "", local_file: false, icon: "zookeeper", driver_profile: None },
    DatabaseTypeSpec { key: "nacos", label: "Nacos", category: DatabaseCategory::RegistryConfig, db_type: "nacos", default_port: 8848, default_user: "nacos", local_file: false, icon: "nacos", driver_profile: None },
    DatabaseTypeSpec { key: "consul", label: "Consul", category: DatabaseCategory::RegistryConfig, db_type: "consul", default_port: 8500, default_user: "", local_file: false, icon: "consul", driver_profile: None },
];

fn database_type(key: &str) -> DatabaseTypeSpec {
    DATABASE_TYPES
        .iter()
        .find(|spec| spec.key == key)
        .copied()
        .unwrap_or(DATABASE_TYPES[0])
}

struct WidthDialog {
    target: WidthTarget,
    input: Entity<InputState>,
    error: Option<String>,
}

enum ConnectionDialogStep {
    Pick,
    Form,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConnectionDialogBusy {
    Test,
    Save,
}

struct NewConnectionDialog {
    id: String,
    step: ConnectionDialogStep,
    category: DatabaseCategory,
    selected: DatabaseTypeSpec,
    search: Entity<InputState>,
    name: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    database: Entity<InputState>,
    ssl: bool,
    save_password: bool,
    busy: Option<ConnectionDialogBusy>,
    result: Option<(bool, String)>,
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
    running_query: Option<String>,
    cancel_requested: Option<String>,
    grid: Option<Entity<TableState<GridDelegate>>>,
    queries: Vec<QueryTab>,
    active_query: Option<usize>,
    next_query: u64,
    show_query: bool,
    schemas: HashMap<String, Vec<String>>,
    result_tab: ResultTab,
    query_info: Option<QueryInfo>,
    query_edit: Option<QueryEditTarget>,
    query_split: f32,
    query_bounds: Rc<Cell<gpui_kit::Bounds<gpui_kit::Pixels>>>,
    query_connection_open: bool,
    query_database_open: bool,
    query_schema_open: bool,
    search: Entity<InputState>,
    filter: String,
    sidebar_open: bool,
    notice: Option<String>,
    notice_id: u64,
    tools_open: bool,
    query_ms: Option<u128>,
    column_meta: Vec<ColumnMeta>,
    show_comments: bool,
    grid_show_types: bool,
    grid_zoom: f32,
    grid_row_height: f32,
    grid_viewport: Rc<Cell<f32>>,
    column_vis_open: bool,
    width_dialog: Option<WidthDialog>,
    connection_dialog: Option<NewConnectionDialog>,
    pending_edits: HashMap<(usize, usize), PendingEdit>,
    new_rows: HashSet<usize>,
    pending_deletes: HashMap<usize, Vec<(String, String, String)>>,
    saving: bool,
    update: Option<UpdateInfo>,
    search_open: bool,
    filters_visible: bool,
    filter_split: f32,
    filter_bounds: Rc<Cell<gpui_kit::Bounds<gpui_kit::Pixels>>>,
    props_open: bool,
    props_anim: f32,
    props_anim_epoch: u64,
    props_split: f32,
    content_bounds: Rc<Cell<gpui_kit::Bounds<gpui_kit::Pixels>>>,
    props_tab: PropsTab,
    props_loading: bool,
    props_error: Option<String>,
    props: TableProps,
    props_editor: Option<Entity<EditorState>>,
    props_target: Option<String>,
    language: String,
    focus_handle: FocusHandle,
    tree_scroll: ScrollHandle,
    tab_scroll: ScrollHandle,
    props_tab_scroll: ScrollHandle,
    tree_row_count: std::cell::Cell<usize>,
    table_where: Entity<InputState>,
    table_order: Entity<InputState>,
    table_page: usize,
    table_has_more: bool,
    where_value: String,
    where_open: bool,
    where_suppress_open: bool,
    where_selected: usize,
    order_value: String,
    order_open: bool,
    order_suppress_open: bool,
    order_selected: usize,
}

fn split_handle() -> ResizeHandleRenderer {
    Rc::new(|_, _, _| Some(div().into_any_element()))
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
            .on_action(cx.listener(|_, _: &ConfirmEnter, window, cx| {
                window.dispatch_action(
                    Box::new(Enter {
                        secondary: false,
                        shift: false,
                    }),
                    cx,
                );
            }))
            .on_action(cx.listener(|_, _: &ConfirmTab, window, cx| {
                window.dispatch_action(Box::new(IndentInline), cx);
            }))
            .on_action(cx.listener(|_, _: &SuggestUp, window, cx| {
                window.dispatch_action(Box::new(MoveUp), cx);
            }))
            .on_action(cx.listener(|_, _: &SuggestDown, window, cx| {
                window.dispatch_action(Box::new(MoveDown), cx);
            }))
            .child(shell)
            .when(self.column_vis_open, |this| {
                this.child(self.column_visibility_overlay(cx))
            })
            .when(self.width_dialog.is_some(), |this| {
                this.child(self.width_dialog_overlay(cx))
            })
            .when(self.connection_dialog.is_some(), |this| {
                this.child(self.connection_dialog_overlay(cx))
            })
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
    let backend = backend::Backend::start()
        .map_err(|error| eprintln!("OMDBX backend startup failed: {error}"))
        .ok();
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx: &mut App| {
        gpui_kit::init(cx);
        if let Some(backend) = backend {
            cx.on_app_quit(move |_| {
                backend.stop();
                async {}
            })
            .detach();
        }
        cx.bind_keys([
            gpui_kit::KeyBinding::new("cmd-r", RefreshConnections, None),
            gpui_kit::KeyBinding::new("tab", ConfirmTab, Some("Input")),
            gpui_kit::KeyBinding::new("enter", ConfirmEnter, Some("Input")),
            gpui_kit::KeyBinding::new("shift-enter", ConfirmEnter, Some("Input")),
            gpui_kit::KeyBinding::new("up", SuggestUp, Some("Input")),
            gpui_kit::KeyBinding::new("down", SuggestDown, Some("Input")),
        ]);
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

#[cfg(test)]
mod export_tests {
    use super::*;

    fn connection() -> Connection {
        Connection {
            id: "test".into(),
            name: "test".into(),
            db_type: "mysql".into(),
            driver_profile: None,
            default_schema: None,
        }
    }

    fn data() -> TableData {
        TableData {
            columns: vec!["id".into(), "name".into(), "note".into()],
            column_types: vec!["int".into(), "varchar".into(), "text".into()],
            rows: vec![
                vec!["1".into(), "Ann, \"A\"".into(), "NULL".into()],
                vec!["2".into(), "Bob".into(), "line\nnext".into()],
            ],
            has_more: false,
        }
    }

    #[test]
    fn sqlformatter_keeps_short_clauses_inline() {
        let out = format_sql_text(
            "select id, name from customers where id > 10 order by id limit 50",
            "mysql",
        );
        assert_eq!(
            out,
            "SELECT id, name\nFROM customers\nWHERE id > 10\nORDER BY id\nLIMIT 50"
        );
    }

    #[test]
    fn explain_sql_dispatches_sqlite_query_plan() {
        let mut connection = connection();
        assert_eq!(
            explain_sql(&connection, "SELECT id FROM people;"),
            "EXPLAIN SELECT id FROM people"
        );
        connection.db_type = "sqlite".into();
        assert_eq!(
            explain_sql(&connection, "SELECT id FROM people;"),
            "EXPLAIN QUERY PLAN SELECT id FROM people"
        );
        assert_eq!(
            explain_sql(&connection, "EXPLAIN SELECT id FROM people"),
            "EXPLAIN SELECT id FROM people"
        );
    }

    #[test]
    fn backend_errors_surface_the_server_detail() {
        assert_eq!(
            backend_error_message(r#"{"detail":"connection refused"}"#, 500),
            "connection refused"
        );
        assert_eq!(backend_error_message("", 502), "请求失败（HTTP 502）");
    }

    #[test]
    fn export_file_stem_rejects_path_characters() {
        assert_eq!(safe_export_stem("reports/2026:11"), "reports_2026_11");
        assert_eq!(safe_export_stem(" .. "), "export");
    }

    #[test]
    fn csv_quotes_and_keeps_nulls() {
        let csv = export_csv(&data());
        assert!(csv.contains("\"Ann, \"\"A\"\"\""));
        assert!(csv.contains("\"line\nnext\""));
        assert!(csv.contains(",NULL\n"));
    }

    #[test]
    fn json_uses_column_names() {
        let value: serde_json::Value = serde_json::from_str(&export_json(&data()).unwrap()).unwrap();
        assert_eq!(value[0]["name"], "Ann, \"A\"");
        assert_eq!(value[1]["note"], "line\nnext");
    }

    #[test]
    fn sql_insert_quotes_strings_and_keeps_numbers() {
        let sql = export_sql_insert(&data(), "people", &connection());
        assert!(sql.contains("INSERT INTO `people` (`id`, `name`, `note`) VALUES (1, 'Ann, \"A\"', NULL);"));
        assert!(sql.contains("'line\nnext'"));
    }

    #[test]
    fn update_uses_primary_keys_and_escapes_strings() {
        let keys = vec![("id".to_string(), "int".to_string(), "1".to_string())];
        let assignments = vec![("name".to_string(), "varchar".to_string(), "O'Brien".to_string())];
        let sql = update_row_sql(&connection(), "people", &assignments, &keys).unwrap();
        assert_eq!(
            sql,
            "UPDATE `people` SET `name` = 'O''Brien' WHERE `id` = 1"
        );
    }

    #[test]
    fn insert_and_delete_sql_cover_pending_rows() {
        let meta = vec![
            ColumnMeta {
                name: "id".to_string(),
                data_type: "int".to_string(),
                comment: String::new(),
                is_primary_key: true,
                is_nullable: false,
                column_default: None,
                extra: None,
            },
            ColumnMeta {
                name: "name".to_string(),
                data_type: "varchar".to_string(),
                comment: String::new(),
                is_primary_key: false,
                is_nullable: true,
                column_default: None,
                extra: None,
            },
            ColumnMeta {
                name: "note".to_string(),
                data_type: "varchar".to_string(),
                comment: String::new(),
                is_primary_key: false,
                is_nullable: true,
                column_default: None,
                extra: None,
            },
        ];
        let columns = vec!["id".to_string(), "name".to_string(), "note".to_string()];
        let values = vec![String::new(), "A".to_string(), "NULL".to_string()];
        let sql = insert_row_sql(&connection(), "people", &columns, &meta, &values).unwrap();
        assert_eq!(
            sql,
            "INSERT INTO `people` (`name`, `note`) VALUES ('A', NULL)"
        );
        let values = vec![String::new(), String::new(), String::new()];
        let sql = insert_row_sql(&connection(), "people", &columns, &meta, &values).unwrap();
        assert_eq!(sql, "INSERT INTO `people` () VALUES ()");
        let keys = vec![("id".to_string(), "int".to_string(), "1".to_string())];
        assert_eq!(
            delete_row_sql(&connection(), "people", &keys).unwrap(),
            "DELETE FROM `people` WHERE `id` = 1"
        );
    }

    #[test]
    fn date_picker_formats_follow_the_language() {
        assert_eq!(date_picker_format("zh-CN", false), "%Y-%m-%d");
        assert_eq!(date_picker_format("en", false), "%m/%d/%Y");
        assert_eq!(date_picker_format("en", true), "%m/%d/%Y %H:%M:%S");
        assert_eq!(date_picker_format("ru", false), "%d.%m.%Y");
        assert_eq!(date_picker_format("pt-BR", false), "%d/%m/%Y");
    }

    #[test]
    fn temporal_editors_pick_the_right_widget() {
        assert_eq!(temporal_kind("date"), TemporalKind::Date);
        assert_eq!(temporal_kind("datetime"), TemporalKind::DateTime);
        assert_eq!(
            temporal_kind("timestamp without time zone"),
            TemporalKind::DateTime
        );
        assert_eq!(temporal_kind("time"), TemporalKind::Time);
        assert_eq!(temporal_kind("varchar(20)"), TemporalKind::Text);
        let value = parse_datetime("2025-11-05 22:38:02").unwrap();
        assert_eq!(value.to_string(), "2025-11-05 22:38:02");
        assert_eq!(parse_datetime("2025-11-05").unwrap().to_string(), "2025-11-05 00:00:00");
        assert_eq!(parse_time("09:30:15").unwrap().to_string(), "09:30:15");
        assert!(parse_time("NULL").is_none());
    }

    #[test]
    fn update_row_groups_columns_into_one_statement() {
        let keys = vec![("id".to_string(), "int".to_string(), "3".to_string())];
        let assignments = vec![
            ("name".to_string(), "varchar".to_string(), "A".to_string()),
            ("note".to_string(), "varchar".to_string(), "NULL".to_string()),
        ];
        let sql = update_row_sql(&connection(), "people", &assignments, &keys).unwrap();
        assert_eq!(
            sql,
            "UPDATE `people` SET `name` = 'A', `note` = NULL WHERE `id` = 3"
        );
    }

    #[test]
    fn update_matches_a_null_key_and_rejects_keyless_tables() {
        let keys = vec![("id".to_string(), "int".to_string(), "NULL".to_string())];
        let assignments = vec![("name".to_string(), "varchar".to_string(), "x".to_string())];
        let sql = update_row_sql(&connection(), "people", &assignments, &keys).unwrap();
        assert!(sql.ends_with("WHERE `id` IS NULL"));
        assert!(update_row_sql(&connection(), "people", &assignments, &[]).is_err());
    }

    #[test]
    fn xlsx_is_a_zip_container() {
        let path = std::env::temp_dir().join("omdbx-export-test.xlsx");
        write_export(&path, ExportFormat::Xlsx, &data(), "people", &connection()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..2], b"PK");
        std::fs::remove_file(path).ok();
    }
}
