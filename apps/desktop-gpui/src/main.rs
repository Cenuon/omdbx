//! OMDBX desktop, Rust + GPUI, styled with GPUI Kit.
//! Talks to the existing `dbx-web` sidecar over HTTP so all connection and query
//! logic is shared with the Electron build.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_xlsxwriter::Workbook;

use gpui_kit::component::button::{Button, ButtonVariants};
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
use gpui_kit::base::{h_resizable, resizable_panel, ResizeHandleRenderer};
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

const API: &str = "http://127.0.0.1:4225";
const ROW_LIMIT: usize = 1000;
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
    #[serde(default)]
    is_primary_key: bool,
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

struct GridDelegate {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    meta: Vec<ColumnMeta>,
    show_comments: bool,
    row_offset: usize,
    editing: Option<EditingCell>,
    /// Present for table tabs; query-result grids are read-only.
    weak: Option<gpui_kit::WeakEntity<Omdbx>>,
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
        let mut width = self
            .meta
            .get(index - 1)
            .map(|meta| {
                let text = (meta.name.chars().count() as f32 * 8.)
                    .max(meta.data_type.chars().count() as f32 * 7.)
                    .max(meta.comment.chars().count() as f32 * 11.);
                text + 28.
            })
            .unwrap_or(140.);
        if let Some(meta) = self.meta.get(index - 1) {
            if is_temporal_type(&meta.data_type) {
                // Dates and timestamps must read in full, so size the column
                // from its longest value instead of the column name.
                let longest = self
                    .rows
                    .iter()
                    .filter_map(|row| row.get(index - 1))
                    .map(|value| value.chars().count())
                    .max()
                    .unwrap_or(0);
                width = width.max(longest as f32 * 8. + 28.).max(160.);
            }
        }
        Column::new(name.clone(), name)
            .width(px(width.clamp(120., 300.)))
            .paddings(padding)
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        // Stripe only real rows; the component's own stripe option also paints
        // the filler rows below the last record.
        div()
            .id(("row", row_ix))
            .when(row_ix % 2 == 1, |this| this.bg(cx.theme().tokens.table_even))
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
            .font_family(cx.theme().mono_font_family.clone())
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
                .font_family(cx.theme().mono_font_family.clone())
                .font_features(tabular())
                .child(format!("{}", self.row_offset + row + 1))
                .into_any_element();
        }
        if let Some(editing) = &self.editing {
            if editing.row == row && editing.col == col - 1 {
                let text_framed = matches!(&editing.editor, CellEditor::Text(_));
                let content = match &editing.editor {
                    CellEditor::Text(input) => Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .w_full()
                        .with_size(gpui_kit::component::Size::XSmall)
                        .text_size(px(13.))
                        .font_family(cx.theme().mono_font_family.clone())
                        .into_any_element(),
                    CellEditor::Date(state) => DatePicker::new(state)
                        .appearance(false)
                        .with_size(gpui_kit::component::Size::XSmall)
                        .w_full()
                        .into_any_element(),
                    CellEditor::Time(state) => TimeField::new(state)
                        .with_size(gpui_kit::component::Size::XSmall)
                        .into_any_element(),
                };
                return div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .rounded(px(2.))
                    .when(text_framed, |this| {
                        this.border_1()
                            .border_color(cx.theme().colors.foreground)
                    })
                    .bg(cx.theme().colors.background)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(content)
                    .into_any_element();
            }
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
        let column_name = self.columns.get(col - 1).cloned().unwrap_or_default();
        let cell = div()
            .id(SharedString::from(format!("cell-body-{row}-{col}")))
            .h_full()
            .w_full()
            .flex()
            .items_center()
            .font_family(cx.theme().mono_font_family.clone())
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
                    .child(text.clone()),
            );
        if let Some(weak) = self.weak.clone() {
            let table = cx.entity();
            let data_col = col - 1;
            return cell
                .context_menu(move |menu, window, cx| {
                    cell_context_menu(
                        menu,
                        window,
                        cx,
                        weak.clone(),
                        table.clone(),
                        row,
                        data_col,
                        column_name.clone(),
                        text.clone(),
                    )
                })
                .into_any_element();
        }
        cell.into_any_element()
    }
}

fn cell_context_menu(
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    weak: gpui_kit::WeakEntity<Omdbx>,
    table: Entity<TableState<GridDelegate>>,
    row: usize,
    col: usize,
    column: String,
    value: String,
) -> PopupMenu {
    let mut menu = menu;

    let setter = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("设置为空白字符串").on_click(move |_, _, cx| {
            let _ = setter.update(cx, |this, cx| {
                this.stage_cell_value(row, col, "", cx)
            });
        }),
    );
    let setter = weak.clone();
    menu = menu.item(PopupMenuItem::new("设置为 NULL").on_click(move |_, _, cx| {
        let _ = setter.update(cx, |this, cx| {
            this.stage_cell_value(row, col, "NULL", cx)
        });
    }));

    let uuid_weak = weak.clone();
    menu = menu.submenu("生成 UUID", window, cx, move |menu, _, _| {
        let v4 = uuid_weak.clone();
        let v7 = uuid_weak.clone();
        menu.item(PopupMenuItem::new("UUID v4").on_click(move |_, _, cx| {
            let value = uuid::Uuid::new_v4().to_string();
            let _ = v4.update(cx, |this, cx| {
                this.stage_cell_value(row, col, &value, cx)
            });
        }))
        .item(PopupMenuItem::new("UUID v7").on_click(move |_, _, cx| {
            let value = uuid::Uuid::now_v7().to_string();
            let _ = v7.update(cx, |this, cx| {
                this.stage_cell_value(row, col, &value, cx)
            });
        }))
    });

    let edit = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("在单元格编辑器中编辑").on_click(move |_, window, cx| {
            let _ = edit.update(cx, |this, cx| {
                this.begin_cell_edit(row, col + 1, &table, window, cx)
            });
        }),
    );

    let add = weak.clone();
    let remove = weak.clone();
    menu = menu
        .separator()
        .item(PopupMenuItem::new("添加记录").on_click(move |_, _, cx| {
            let _ = add.update(cx, |this, cx| this.show_notice("添加记录未完成", cx));
        }))
        .item(PopupMenuItem::new("删除记录...").on_click(move |_, _, cx| {
            let _ =
                remove.update(cx, |this, cx| this.show_notice("删除记录未完成", cx));
        }));

    let copy = value.clone();
    let copy_name = column.clone();
    menu = menu
        .separator()
        .item(PopupMenuItem::new("复制").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(copy.clone()));
        }))
        .item(
            PopupMenuItem::new("复制字段名称").on_click(move |_, _, cx| {
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                    copy_name.clone(),
                ));
            }),
        );

    let copy_value = value.clone();
    menu = menu.submenu("复制为", window, cx, move |menu, _, _| {
        let csv = csv_field(&copy_value);
        let json = serde_json::to_string(&copy_value).unwrap_or_default();
        let sql = format!("'{}'", copy_value.replace('\'', "''"));
        menu.item(PopupMenuItem::new("CSV").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(csv.clone()));
        }))
        .item(PopupMenuItem::new("JSON").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                json.clone(),
            ));
        }))
        .item(PopupMenuItem::new("SQL").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(sql.clone()));
        }))
    });

    let paste = weak.clone();
    menu = menu.item(PopupMenuItem::new("粘贴").on_click(move |_, _, cx| {
        let text = cx
            .read_from_clipboard()
            .and_then(|item| item.text());
        match text {
            Some(text) => {
                let _ = paste.update(cx, |this, cx| {
                    this.stage_cell_value(row, col, &text, cx)
                });
            }
            None => {
                let _ =
                    paste.update(cx, |this, cx| this.show_notice("剪贴板没有文本", cx));
            }
        }
    }));

    let save = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("保存数据为...").on_click(move |_, _, cx| {
            let _ = save.update(cx, |this, cx| {
                this.export_table(ExportFormat::Csv, true, cx)
            });
        }),
    );

    menu = menu.separator();
    let toggle = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("显示/隐藏筛选&排序").on_click(move |_, _, cx| {
            let _ = toggle.update(cx, |this, cx| {
                this.filters_visible = !this.filters_visible;
                cx.notify();
            });
        }),
    );

    let filter = weak.clone();
    let clear_filter = weak.clone();
    menu = menu.submenu("筛选", window, cx, move |menu, _, _| {
        let by_value = filter.clone();
        let clear = clear_filter.clone();
        menu.item(
            PopupMenuItem::new("按此值筛选").on_click(move |_, window, cx| {
                let _ = by_value.update(cx, |this, cx| {
                    this.filter_by_cell(row, col, window, cx)
                });
            }),
        )
        .item(PopupMenuItem::new("清空筛选").on_click(move |_, window, cx| {
            let _ = clear.update(cx, |this, cx| this.clear_where(window, cx));
        }))
    });

    let sort = weak.clone();
    menu = menu.submenu("排序", window, cx, move |menu, _, _| {
        let asc = sort.clone();
        let desc = sort.clone();
        menu.item(PopupMenuItem::new("升序").on_click(move |_, window, cx| {
            let _ = asc.update(cx, |this, cx| this.sort_by_cell(col, true, window, cx));
        }))
        .item(PopupMenuItem::new("降序").on_click(move |_, window, cx| {
            let _ =
                desc.update(cx, |this, cx| this.sort_by_cell(col, false, window, cx));
        }))
    });

    let clear_all = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("移除所有筛选 & 排序").on_click(move |_, window, cx| {
            let _ =
                clear_all.update(cx, |this, cx| this.remove_filters(window, cx));
        }),
    );

    let zoom = weak.clone();
    menu = menu.separator().submenu("缩放", window, cx, move |menu, _, _| {
        let w100 = zoom.clone();
        let w125 = zoom.clone();
        let w150 = zoom.clone();
        menu.item(PopupMenuItem::new("100%").on_click(move |_, _, cx| {
            let _ = w100.update(cx, |this, cx| this.show_notice("缩放未完成", cx));
        }))
        .item(PopupMenuItem::new("125%").on_click(move |_, _, cx| {
            let _ = w125.update(cx, |this, cx| this.show_notice("缩放未完成", cx));
        }))
        .item(PopupMenuItem::new("150%").on_click(move |_, _, cx| {
            let _ = w150.update(cx, |this, cx| this.show_notice("缩放未完成", cx));
        }))
    });

    let refresh = weak.clone();
    menu.item(PopupMenuItem::new("刷新").on_click(move |_, window, cx| {
        let _ = refresh.update(cx, |this, cx| this.load_active(window, cx));
    }))
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
    search: Entity<InputState>,
    filter: String,
    sidebar_open: bool,
    notice: Option<String>,
    notice_id: u64,
    tools_open: bool,
    query_ms: Option<u128>,
    column_meta: Vec<ColumnMeta>,
    show_comments: bool,
    pending_edits: HashMap<(usize, usize), PendingEdit>,
    saving: bool,
    update: Option<UpdateInfo>,
    search_open: bool,
    filters_visible: bool,
    focus_handle: FocusHandle,
    tree_scroll: ScrollHandle,
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

impl Omdbx {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            search,
            filter: String::new(),
            sidebar_open: true,
            notice: None,
            notice_id: 0,
            tools_open: false,
            query_ms: None,
            column_meta: Vec::new(),
            show_comments: true,
            pending_edits: HashMap::new(),
            saving: false,
            update: None,
            search_open: false,
            filters_visible: true,
            focus_handle: cx.focus_handle(),
            tree_scroll: ScrollHandle::new(),
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
        self.clear_table_filters(window, cx);
        self.load_active(window, cx);
    }

    fn clear_table_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.table_page = 1;
        self.table_has_more = false;
        self.table_where
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.table_order
            .update(cx, |state, cx| state.set_value("", window, cx));
    }

    fn load_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A reload rebuilds the rows, so staged edits no longer map to them.
        if !self.pending_edits.is_empty() {
            self.pending_edits.clear();
            self.show_notice("未保存的修改已丢弃", cx);
        }
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
                        let delegate = GridDelegate {
                            columns: data.columns.clone(),
                            rows: data.rows.clone(),
                            meta,
                            show_comments: this.show_comments,
                            row_offset,
                            editing: None,
                            weak: Some(cx.entity().downgrade()),
                        };
                        this.grid = Some(cx.new(|cx| {
                            TableState::new(delegate, window, cx)
                                .col_resizable(false)
                                .cell_selectable(true)
                                .row_header(false)
                        }));
                        if let Some(grid) = this.grid.clone() {
                            cx.subscribe_in(
                                &grid,
                                window,
                                |this, grid, event: &TableEvent, window, cx| {
                                    if let TableEvent::DoubleClickedCell(row, col) = event {
                                        this.begin_cell_edit(*row, *col, grid, window, cx);
                                    }
                                },
                            )
                            .detach();
                        }
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

    fn stop_query(&mut self, cx: &mut Context<Self>) {
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

    fn cancel_running_query(&mut self) {
        if let Some(execution_id) = self.running_query.take() {
            Self::cancel_query_id(execution_id);
        }
        self.cancel_requested = None;
        self.loading = false;
    }

    fn cancel_query_id(execution_id: String) {
        std::thread::spawn(move || {
            let _ = post_json(
                &format!("{API}/api/query/cancel"),
                &json!({ "executionId": execution_id }),
            );
        });
    }

    fn begin_cell_edit(
        &mut self,
        row: usize,
        col: usize,
        grid: &Entity<TableState<GridDelegate>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if col == 0 || self.show_query {
            return;
        }
        let col = col - 1;
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
        let format = if with_time {
            "%Y-%m-%d %H:%M:%S"
        } else {
            "%Y-%m-%d"
        };
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

    fn commit_editing(&mut self, grid: &Entity<TableState<GridDelegate>>, cx: &mut Context<Self>) {
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

    fn finish_edit(
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

    fn set_local_cell(&mut self, row: usize, col: usize, value: &str, cx: &mut Context<Self>) {
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

    fn primary_key_values(
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

    fn row_keys(
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

    fn stage_cell_value(
        &mut self,
        row: usize,
        col: usize,
        value: &str,
        cx: &mut Context<Self>,
    ) {
        if self.show_query {
            return;
        }
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            return;
        };
        let page = self.table_page;
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
            .filter(|edit| edit.tab_key == tab.key && edit.page == page)
            .map(|edit| edit.original.clone())
            .unwrap_or(current);
        let keys = match self.row_keys(row, &tab.key, page) {
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
                    tab_key: tab.key.clone(),
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
        cx.notify();
    }

    fn cancel_pending_edits(&mut self, cx: &mut Context<Self>) {
        if let Some(grid) = self.grid.clone() {
            grid.update(cx, |table, cx| {
                table.delegate_mut().editing = None;
                cx.notify();
            });
        }
        let tab_key = self
            .active
            .and_then(|index| self.tabs.get(index))
            .map(|tab| tab.key.clone());
        let page = self.table_page;
        let entries: Vec<((usize, usize), PendingEdit)> = self
            .pending_edits
            .iter()
            .map(|(key, edit)| (*key, edit.clone()))
            .collect();
        let mut restored = 0;
        for ((row, col), edit) in entries {
            if tab_key.as_deref() == Some(edit.tab_key.as_str()) && edit.page == page {
                self.set_local_cell(row, col, &edit.original, cx);
                restored += 1;
            }
        }
        self.pending_edits.clear();
        cx.notify();
        if restored == 0 {
            self.show_notice("没有待取消的修改", cx);
        } else {
            self.show_notice("已取消修改", cx);
        }
    }

    fn save_pending_edits(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        if let Some(grid) = self.grid.clone() {
            self.commit_editing(&grid, cx);
        }
        let Some(tab) = self.active.and_then(|index| self.tabs.get(index)).cloned() else {
            return;
        };
        let page = self.table_page;
        self.pending_edits
            .retain(|_, edit| edit.tab_key == tab.key && edit.page == page);
        if self.pending_edits.is_empty() {
            self.show_notice("没有需要保存的修改", cx);
            return;
        }
        let Some(connection) = self
            .connections
            .iter()
            .find(|item| item.id == tab.connection_id)
            .cloned()
        else {
            return;
        };
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
        let mut statements = Vec::with_capacity(grouped.len());
        for (_, (assignments, keys)) in grouped {
            match update_row_sql(&connection, &tab.table, &assignments, &keys) {
                Ok(sql) => statements.push(sql),
                Err(message) => {
                    self.show_notice(&message, cx);
                    return;
                }
            }
        }
        let saved_keys: Vec<(usize, usize)> = self.pending_edits.keys().copied().collect();
        let connection = connection.clone();
        let database = tab.database.clone();
        let schema = tab.schema.clone();
        self.saving = true;
        let task = cx.background_executor().spawn(async move {
            for sql in statements {
                run_sql(&connection, &database, &schema, &sql, 1, None)?;
            }
            Ok::<(), String>(())
        });
        cx.spawn(async move |weak, cx| {
            let result = task.await;
            let _ = weak.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(()) => {
                        for key in &saved_keys {
                            this.pending_edits.remove(key);
                        }
                        this.show_notice(
                            &format!("已保存 {} 处修改", saved_keys.len()),
                            cx,
                        );
                    }
                    Err(error) => this.show_notice(&format!("保存失败: {error}"), cx),
                }
            });
        })
        .detach();
    }

    fn filter_by_cell(
        &mut self,
        row: usize,
        col: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn sort_by_cell(
        &mut self,
        col: usize,
        ascending: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn clear_where(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.table_where
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.where_value.clear();
        self.where_open = false;
        self.table_page = 1;
        self.load_active(window, cx);
    }

    fn remove_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn export_table(
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
        let file_name = format!("{}{}.{}", tab.table, suffix, format.extension());
        let directory = std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Downloads"))
            .filter(|path| path.is_dir())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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

    fn open_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_running_query();
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
        if let Some(previous) = self.running_query.take() {
            Self::cancel_query_id(previous);
        }
        self.loading = true;
        let execution_id = uuid::Uuid::new_v4().to_string();
        self.running_query = Some(execution_id.clone());
        self.cancel_requested = None;
        cx.notify();
        let started = std::time::Instant::now();
        let task_id = execution_id.clone();
        let sql_for_tables = sql.clone();
        let task = cx.background_executor().spawn(async move {
            let data = run_sql(
                &connection,
                &database,
                &schema,
                &sql,
                ROW_LIMIT,
                Some(&task_id),
            )?;
            let mut schema_columns: Vec<ColumnMeta> = Vec::new();
            for table in sql_tables(&sql_for_tables) {
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
            Ok::<(TableData, Vec<ColumnMeta>), String>((data, schema_columns))
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
                    Ok((data, schema_columns)) => {
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
                        this.column_meta = meta.clone();
                        let delegate = GridDelegate {
                            columns: data.columns.clone(),
                            rows: data.rows.clone(),
                            meta,
                            show_comments: this.show_comments,
                            row_offset: 0,
                            editing: None,
                            weak: None,
                        };
                        this.grid = Some(cx.new(|cx| {
                            TableState::new(delegate, window, cx).col_resizable(false)
                        }));
                        this.columns = data.columns;
                        this.rows = data.rows;
                        this.error = None;
                        this.query_ms = Some(started.elapsed().as_millis());
                    }
                    Err(message) => {
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
            self.cancel_running_query();
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

    fn close_others(&mut self, target: CloseTarget, window: &mut Window, cx: &mut Context<Self>) {
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

    fn close_right(&mut self, target: CloseTarget, window: &mut Window, cx: &mut Context<Self>) {
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

    fn close_all_tabs(&mut self, cx: &mut Context<Self>) {
        self.cancel_running_query();
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

    fn reveal_in_tree(&mut self, target: CloseTarget, window: &mut Window, cx: &mut Context<Self>) {
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
        if !self.filter.trim().is_empty() || self.tree_scroll.max_offset().y <= px(0.) {
            return Vec::new();
        }
        // Nothing is scrolled past the first row: keep the list unadorned.
        let top_index = self.tree_scroll.top_item();
        if top_index == 0 {
            return Vec::new();
        }
        let (connection_id, database) = match entries.get(top_index) {
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
                                .border_b_1()
                                .border_color(colors.border.opacity(0.4))
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

    fn suggestions_for(&self, value: &str) -> Vec<Suggestion> {
        let word = last_word(value);
        if word.is_empty() {
            return Vec::new();
        }
        let needle = word.to_lowercase();
        // Lower rank wins: field prefix, keyword prefix, field substring,
        // keyword substring.
        let mut ranked: Vec<(usize, String, Suggestion)> = Vec::new();
        for (index, column) in self.columns.iter().enumerate() {
            let lower = column.to_lowercase();
            let Some(position) = lower.find(&needle) else {
                continue;
            };
            let rank = if position == 0 { 0 } else { 2 };
            let detail = self
                .column_meta
                .get(index)
                .map(|meta| {
                    if meta.comment.is_empty() {
                        meta.data_type.clone()
                    } else {
                        meta.comment.clone()
                    }
                })
                .unwrap_or_default();
            ranked.push((
                rank,
                lower,
                Suggestion {
                    text: column.clone(),
                    detail,
                },
            ));
        }
        for keyword in SQL_KEYWORDS {
            let lower = keyword.to_lowercase();
            let Some(position) = lower.find(&needle) else {
                continue;
            };
            let rank = if position == 0 { 1 } else { 3 };
            ranked.push((
                rank,
                lower,
                Suggestion {
                    text: keyword.to_string(),
                    detail: "SQL".to_string(),
                },
            ));
        }
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.text.cmp(&b.2.text)));
        ranked
            .into_iter()
            .take(8)
            .map(|(_, _, suggestion)| suggestion)
            .collect()
    }

    fn suggestion_selection(&self, field: SuggestField) -> usize {
        match field {
            SuggestField::Where => self.where_selected,
            SuggestField::Order => self.order_selected,
        }
    }

    fn set_suggestion_selection(&mut self, field: SuggestField, index: usize) {
        match field {
            SuggestField::Where => self.where_selected = index,
            SuggestField::Order => self.order_selected = index,
        }
    }

    fn close_suggestions(&mut self, field: SuggestField) {
        match field {
            SuggestField::Where => self.where_open = false,
            SuggestField::Order => self.order_open = false,
        }
    }

    fn pick_suggestion(
        &mut self,
        field: SuggestField,
        column: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (state, value) = match field {
            SuggestField::Where => (self.table_where.clone(), self.where_value.clone()),
            SuggestField::Order => (self.table_order.clone(), self.order_value.clone()),
        };
        let word = last_word(&value);
        let base = value.strip_suffix(&word).unwrap_or(&value).to_string();
        match field {
            SuggestField::Where => self.where_suppress_open = true,
            SuggestField::Order => self.order_suppress_open = true,
        }
        let completed = format!("{base}{column}");
        state.update(cx, |state, cx| state.set_value(completed.clone(), window, cx));
        // set_value does not emit Change for every input mode; keep the
        // painted layer in step with the real value.
        match field {
            SuggestField::Where => self.where_value = completed,
            SuggestField::Order => self.order_value = completed,
        }
        match field {
            SuggestField::Where => {
                self.where_suppress_open = false;
                self.where_open = false;
            }
            SuggestField::Order => {
                self.order_suppress_open = false;
                self.order_open = false;
            }
        }
        cx.notify();
    }

    fn handle_suggestion_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let where_suggestions = if self.where_open {
            self.suggestions_for(&self.where_value)
        } else {
            Vec::new()
        };
        let order_suggestions = if self.order_open {
            self.suggestions_for(&self.order_value)
        } else {
            Vec::new()
        };
        let (field, suggestions) = if !where_suggestions.is_empty() {
            (SuggestField::Where, where_suggestions)
        } else if !order_suggestions.is_empty() {
            (SuggestField::Order, order_suggestions)
        } else {
            if key == "enter" {
                self.load_active(window, cx);
                return true;
            }
            return false;
        };
        let selected = self
            .suggestion_selection(field)
            .min(suggestions.len() - 1);
        match key {
            "up" => {
                self.set_suggestion_selection(field, selected.saturating_sub(1));
                cx.notify();
                true
            }
            "down" => {
                self.set_suggestion_selection(field, (selected + 1).min(suggestions.len() - 1));
                cx.notify();
                true
            }
            "enter" | "tab" => {
                let column = suggestions[selected].text.clone();
                self.pick_suggestion(field, &column, window, cx);
                true
            }
            "escape" => {
                self.close_suggestions(field);
                cx.notify();
                true
            }
            _ => false,
        }
    }

    fn suggest_menu(
        &self,
        field: SuggestField,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let (open, value, selected) = match field {
            SuggestField::Where => (self.where_open, self.where_value.as_str(), self.where_selected),
            SuggestField::Order => (self.order_open, self.order_value.as_str(), self.order_selected),
        };
        if !open {
            return None;
        }
        let suggestions = self.suggestions_for(value);
        if suggestions.is_empty() {
            return None;
        }
        let selected = selected.min(suggestions.len() - 1);
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let accent: Hsla = match field {
            SuggestField::Where => gpui_kit::rgb(0x2F6BFF).into(),
            SuggestField::Order => gpui_kit::rgb(0xE8722C).into(),
        };
        let prefix = match field {
            SuggestField::Where => "where",
            SuggestField::Order => "order",
        };
        let needle = last_word(value).to_lowercase();

        Some(
            deferred(
                v_flex()
                    .absolute()
                    .occlude()
                    .top(px(28.))
                    .left_0()
                    .right_0()
                    .p_1()
                    .gap(px(1.))
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.background)
                    .shadow_md()
                    .children(suggestions.into_iter().enumerate().map(
                        |(index, suggestion)| {
                            let text = suggestion.text.clone();
                            let match_at = text.to_lowercase().find(&needle);
                            let name: gpui_kit::AnyElement = match match_at {
                                Some(start) if !needle.is_empty() => {
                                    let end = start + needle.len();
                                    h_flex()
                                        .flex_none()
                                        .child(text[..start].to_string())
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(accent)
                                                .child(text[start..end].to_string()),
                                        )
                                        .child(text[end..].to_string())
                                        .into_any_element()
                                }
                                _ => div().child(text.clone()).into_any_element(),
                            };
                            let insert = text;
                            h_flex()
                                .id(SharedString::from(format!("{prefix}-{insert}")))
                                .h(px(26.))
                                .px_2()
                                .gap_2()
                                .items_center()
                                .rounded_md()
                                .text_size(px(12.))
                                .when(index == selected, |this| this.bg(colors.sidebar_accent))
                                .hover(|style| style.bg(colors.sidebar_accent))
                                .child(name)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(11.))
                                        .text_color(muted)
                                        .child(suggestion.detail),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .h(px(16.))
                                        .px_1()
                                        .flex()
                                        .items_center()
                                        .rounded(px(3.))
                                        .border_1()
                                        .border_color(colors.border)
                                        .text_size(px(9.))
                                        .text_color(muted)
                                        .child("Tab"),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, window, cx| {
                                        this.pick_suggestion(field, &insert, window, cx);
                                    }),
                                )
                        },
                    )),
            )
            .into_any_element(),
        )
    }

    fn filter_slot(
        &self,
        field: SuggestField,
        label: &'static str,
        color: Hsla,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (state, value) = match field {
            SuggestField::Where => (&self.table_where, self.where_value.as_str()),
            SuggestField::Order => (&self.table_order, self.order_value.as_str()),
        };
        h_flex()
            .relative()
            .flex_1()
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

    /// WHERE / ORDER BY input above a table tab, plus copy and refresh.
    fn table_filter_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(self.filter_slot(
                SuggestField::Where,
                "WHERE",
                gpui_kit::rgb(0x2F6BFF).into(),
                cx,
            ))
            .child(divider())
            .child(self.filter_slot(
                SuggestField::Order,
                "ORDER BY",
                gpui_kit::rgb(0xE8722C).into(),
                cx,
            ))
            .child(divider())
            .child(Self::icon_button(
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
                            let mut menu = menu.min_w(px(190.));
                            for format in ExportFormat::ALL {
                                let weak = weak.clone();
                                menu = menu.submenu(format.label(), window, cx, move |menu, _, _| {
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
    }

    fn filter_field(
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

    /// Fixed toolbar under a table: row edits, execution control and paging.
    fn table_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let page = self.table_page;
        let icon_size = px(15.);
        let dirty = !self.pending_edits.is_empty();
        let running = self.loading && self.running_query.is_some();
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

        h_flex()
            .h(px(34.))
            .flex_none()
            .px_3()
            .gap_1()
            .items_center()
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.background)
            .child(Self::icon_button(
                "row-add",
                Icon::new(AssetIcon::Plus)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, _, cx| this.show_notice("新增行未完成", cx)),
            ))
            .child(Self::icon_button(
                "row-remove",
                Icon::new(AssetIcon::Minus)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, _, cx| this.show_notice("删除行未完成", cx)),
            ))
            .child(Self::action_button(
                "row-save",
                Icon::new(AssetIcon::Check)
                    .with_size(icon_size)
                    .text_color(save_color)
                    .into_any_element(),
                dirty,
                colors,
                cx.listener(|this, _, _, cx| this.save_pending_edits(cx)),
            ))
            .child(Self::action_button(
                "row-cancel",
                div()
                    .text_size(px(14.))
                    .text_color(cancel_color)
                    .child("✕")
                    .into_any_element(),
                dirty,
                colors,
                cx.listener(|this, _, _, cx| this.cancel_pending_edits(cx)),
            ))
            .child(Self::icon_button(
                "table-reload",
                Icon::new(AssetIcon::RotateCw)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| this.load_active(window, cx)),
            ))
            .child(Self::action_button(
                "stop-query",
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
            .child(div().flex_1())
            .child(Self::icon_button(
                "page-first",
                Icon::new(AssetIcon::ChevronsLeft)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| {
                    if this.table_page != 1 {
                        this.table_page = 1;
                        this.load_active(window, cx);
                    }
                }),
            ))
            .child(Self::icon_button(
                "page-prev",
                Icon::new(AssetIcon::ChevronLeft)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| {
                    if this.table_page > 1 {
                        this.table_page -= 1;
                        this.load_active(window, cx);
                    }
                }),
            ))
            .child(
                div()
                    .w(px(42.))
                    .h(px(24.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .text_size(px(12.))
                    .child(page.to_string()),
            )
            .child(Self::icon_button(
                "page-next",
                Icon::new(AssetIcon::ChevronRight)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, window, cx| {
                    if this.table_has_more {
                        this.table_page += 1;
                        this.load_active(window, cx);
                    }
                }),
            ))
            .child(Self::icon_button(
                "page-last",
                Icon::new(AssetIcon::ChevronsRight)
                    .with_size(icon_size)
                    .text_color(muted)
                    .into_any_element(),
                colors,
                cx.listener(|this, _, _, cx| {
                    if !this.table_has_more {
                        this.show_notice("已经是最后一页", cx)
                    } else {
                        this.show_notice("跳到最后未完成", cx)
                    }
                }),
            ))
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
                    .child(DataTable::new(grid).bordered(false).small().stripe(false).header_row_height(header_height)),
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
        let running = self.loading && self.running_query.is_some();
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
                    )
                    .child(Self::action_button(
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
                    // the empty strip after the tabs drags the window, like a titlebar
                    .child(
                        div()
                            .id("titlebar-drag")
                            .flex_1()
                            .h_full()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move()
                            }),
                    )
    }

    fn main_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let tab = self.active.and_then(|index| self.tabs.get(index)).cloned();

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
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
            .when(!self.show_query && tab.is_some(), |this| {
                this.child(self.table_footer(cx))
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
                            this.show_query = false;
                            this.active = Some(index);
                            this.clear_table_filters(window, cx);
                            this.load_active(window, cx);
                        }
                        CloseTarget::Query(index) => {
                            this.cancel_running_query();
                            this.show_query = true;
                            this.active_query = Some(index);
                            cx.notify();
                        }
                    }));
            }
            None => {
                chip = chip.on_click(cx.listener(|this, _, _, cx| {
                    this.show_query = true;
                    cx.notify();
                }));
            }
        }
        chip.context_menu(move |menu, _, _| {
            let mut menu = menu;
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

/// Divider between the sidebar and the content: never painted. The 9px band
/// still hit-tests and switches to the column-resize cursor, so the shell keeps
/// its line-free look in every state.
fn split_handle() -> ResizeHandleRenderer {
    Rc::new(|_, _, _| Some(div().into_any_element()))
}

impl Omdbx {
    /// One square icon button: both titlebar buttons go through this so their
    /// hit area, radius and hover fill cannot drift apart.
    fn action_button(
        id: &'static str,
        child: gpui_kit::AnyElement,
        enabled: bool,
        colors: gpui_kit::component::ThemeColor,
        on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let button = h_flex()
            .id(id)
            .h(px(28.))
            .w(px(28.))
            .flex_none()
            .items_center()
            .justify_center()
            .rounded_md()
            .child(child);
        if enabled {
            button
                .cursor_pointer()
                .hover(|style| style.bg(colors.secondary_active))
                .on_click(on_click)
        } else {
            button.opacity(0.35).cursor_default()
        }
    }

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
            host: None,
            port: None,
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
