//! OMDBX desktop, Rust + GPUI. Talks to the existing `dbx-web` sidecar over HTTP,
//! so all connection/query logic is shared with the Electron build.

use gpui::{
    App, Application, Bounds, Context, Render, SharedString, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size,
};
use serde::Deserialize;

const API: &str = "http://127.0.0.1:4224";

const SURFACE: u32 = 0xffffff;
const SIDEBAR: u32 = 0xfafaf8;
const LINE: u32 = 0xe6e6e0;
const INK: u32 = 0x1a1a17;
const INK_2: u32 = 0x4a4a43;
const MUTED: u32 = 0x77776f;
const ACCENT_SOFT: u32 = 0xeef2ee;

#[derive(Debug, Clone, Deserialize)]
struct Connection {
    id: String,
    name: String,
    db_type: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    port: Option<u16>,
}

impl Connection {
    fn endpoint(&self) -> String {
        match (self.host.as_deref(), self.port) {
            (Some(host), Some(port)) if port > 0 => format!("{host}:{port}"),
            (Some(host), _) => host.to_string(),
            _ => "—".to_string(),
        }
    }
}

fn fetch_connections() -> Result<Vec<Connection>, String> {
    let body = ureq::get(&format!("{API}/api/connection/list"))
        .call()
        .map_err(|error| error.to_string())?
        .into_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

struct Omdbx {
    connections: Vec<Connection>,
    selected: Option<String>,
    error: Option<String>,
}

impl Omdbx {
    fn new() -> Self {
        let (connections, error) = match fetch_connections() {
            Ok(list) => (list, None),
            Err(message) => (Vec::new(), Some(message)),
        };
        let selected = connections.first().map(|item| item.id.clone());
        Self {
            connections,
            selected,
            error,
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        match fetch_connections() {
            Ok(list) => {
                self.connections = list;
                self.error = None;
                if !self
                    .connections
                    .iter()
                    .any(|item| Some(&item.id) == self.selected.as_ref())
                {
                    self.selected = self.connections.first().map(|item| item.id.clone());
                }
            }
            Err(message) => self.error = Some(message),
        }
        cx.notify();
    }

    fn selected_connection(&self) -> Option<&Connection> {
        let selected = self.selected.as_ref()?;
        self.connections.iter().find(|item| &item.id == selected)
    }
}

impl Render for Omdbx {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_connection().cloned();

        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(rgb(SURFACE))
            .text_color(rgb(INK))
            .font_family("PingFang SC")
            .child(
                // ---------- sidebar ----------
                div()
                    .flex()
                    .flex_col()
                    .w(px(248.))
                    .h_full()
                    .flex_none()
                    .bg(rgb(SIDEBAR))
                    .border_r_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .h(px(44.))
                            .px_3()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .child(format!("我的连接  {}", self.connections.len())),
                            )
                            .child(
                                div()
                                    .id("refresh")
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .hover(|style| style.bg(rgb(0xf0f0ec)).text_color(rgb(INK)))
                                    .child("刷新")
                                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                            ),
                    )
                    .child(div().flex().flex_col().flex_1().overflow_hidden().children(
                        self.connections.iter().map(|connection| {
                            let is_active = Some(&connection.id) == self.selected.as_ref();
                            let id = connection.id.clone();
                            div()
                                .id(SharedString::from(format!("conn-{}", connection.id)))
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .h(px(30.))
                                .mx_2()
                                .px_2()
                                .rounded_md()
                                .cursor_pointer()
                                .text_size(px(13.))
                                .when(is_active, |this| this.bg(rgb(ACCENT_SOFT)))
                                .when(!is_active, |this| {
                                    this.hover(|style| style.bg(rgb(0xf2f2ee)))
                                })
                                .child(
                                    div()
                                        .text_color(if is_active {
                                            rgb(INK)
                                        } else {
                                            rgb(INK_2)
                                        })
                                        .child(connection.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(MUTED))
                                        .child(connection.db_type.clone()),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.selected = Some(id.clone());
                                    cx.notify();
                                }))
                        }),
                    )),
            )
            .child(
                // ---------- main ----------
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .h(px(44.))
                            .px_4()
                            .border_b_1()
                            .border_color(rgb(LINE))
                            .text_size(px(13.))
                            .child(match &selected {
                                Some(connection) => format!(
                                    "{}   ·   {}",
                                    connection.name,
                                    connection.endpoint()
                                ),
                                None => "未选择连接".to_string(),
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .text_color(rgb(INK_2))
                                    .child("Rust + GPUI 版 · P0"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .child(match &self.error {
                                        Some(message) => format!("读取连接失败：{message}"),
                                        None => "已连上 dbx-web sidecar".to_string(),
                                    }),
                            ),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Omdbx::new()),
        )
        .unwrap();
        cx.activate(true);
    });
}
