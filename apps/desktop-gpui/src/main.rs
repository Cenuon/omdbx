//! OMDBX desktop, Rust + GPUI, styled with GPUI Kit.
//! Talks to the existing `dbx-web` sidecar over HTTP so all connection and query
//! logic is shared with the Electron build.

use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme, Sizable, Theme, ThemeMode};
use gpui_kit::prelude::*;
use gpui_kit::{
    div, px, App, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    WindowOptions,
};
use serde::Deserialize;

const API: &str = "http://127.0.0.1:4224";

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

fn agent() -> ureq::Agent {
    // localhost must never go through the user's HTTP proxy, and a hung sidecar
    // should not freeze the UI thread.
    ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .timeout_connect(Duration::from_secs(2))
        .timeout_read(Duration::from_secs(5))
        .build()
}

fn fetch_connections() -> Result<Vec<Connection>, String> {
    let body = agent()
        .get(&format!("{API}/api/connection/list"))
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
        let colors = cx.theme().colors;
        let selected = self.selected_connection().cloned();

        h_flex()
            .size_full()
            .bg(colors.background)
            .text_color(colors.foreground)
            // ---------- sidebar ----------
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
                                    .text_color(colors.muted_foreground)
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
                            .overflow_hidden()
                            .py_1()
                            .children(self.connections.iter().map(|connection| {
                                let is_active = Some(&connection.id) == self.selected.as_ref();
                                let id = connection.id.clone();
                                let db_type = connection.db_type.clone();
                                ListItem::new(SharedString::from(format!("conn-{}", connection.id)))
                                    .selected(is_active)
                                    .child(div().text_sm().child(connection.name.clone()))
                                    .suffix(move |_, _| {
                                        div()
                                            .text_xs()
                                            .text_color(colors.muted_foreground)
                                            .child(db_type.clone())
                                            .into_any_element()
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.selected = Some(id.clone());
                                        cx.notify();
                                    }))
                            })),
                    ),
            )
            // ---------- main ----------
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .child(
                        h_flex()
                            .h(px(44.))
                            .flex_none()
                            .px_4()
                            .items_center()
                            .gap_2()
                            .border_b_1()
                            .border_color(colors.border)
                            .text_sm()
                            .child(match &selected {
                                Some(connection) => connection.name.clone(),
                                None => "未选择连接".to_string(),
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.muted_foreground)
                                    .child(match &selected {
                                        Some(connection) => connection.endpoint(),
                                        None => String::new(),
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .child(div().text_lg().child("GPUI Kit · P0"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.muted_foreground)
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
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        Theme::update(cx, |theme| theme.mode = ThemeMode::Light);
        gpui_kit::open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::centered(
                    None,
                    gpui_kit::size(px(1000.), px(640.)),
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
