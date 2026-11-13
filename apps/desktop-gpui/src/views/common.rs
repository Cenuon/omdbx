use crate::*;

pub(crate) fn connection_color(key: &str) -> Hsla {
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
        "firebird" => gpui_kit::rgb(0xF05A28),
        "cockroachdb" => gpui_kit::rgb(0x6933FF),
        "db2" => gpui_kit::rgb(0x1F70C1),
        "informix" => gpui_kit::rgb(0x009999),
        "jdbcx" => gpui_kit::rgb(0xE8A317),
        "spanner" => gpui_kit::rgb(0x4285F4),
        "iris" | "cache" => gpui_kit::rgb(0x00A39A),
        "dolt" => gpui_kit::rgb(0x2DAE8D),
        "duckdb" => gpui_kit::rgb(0xFFC300),
        "dm" => gpui_kit::rgb(0xC74634),
        "kingbase" => gpui_kit::rgb(0xE4572E),
        "opengauss" => gpui_kit::rgb(0x2F6BFF),
        "h2" => gpui_kit::rgb(0x1E6FFF),
        "elasticsearch" => gpui_kit::rgb(0xFEC514),
        "easysearch" => gpui_kit::rgb(0x40B5AD),
        "solr" => gpui_kit::rgb(0xD9411E),
        "couchdb" => gpui_kit::rgb(0xE42528),
        "cassandra" => gpui_kit::rgb(0x1287B1),
        "neo4j" => gpui_kit::rgb(0x018BFF),
        "qdrant" => gpui_kit::rgb(0xDC244C),
        "weaviate" => gpui_kit::rgb(0x00A98F),
        "chromadb" => gpui_kit::rgb(0xFF6B35),
        "influxdb" => gpui_kit::rgb(0x22ADF6),
        "tdengine" => gpui_kit::rgb(0x0F6CBD),
        "questdb" => gpui_kit::rgb(0xD14646),
        "iotdb" => gpui_kit::rgb(0x2E86C1),
        "kafka" => gpui_kit::rgb(0x231F20),
        "rocketmq" => gpui_kit::rgb(0xD9534F),
        "rabbitmq" => gpui_kit::rgb(0xFF6600),
        "mqtt" => gpui_kit::rgb(0x660066),
        "etcd" => gpui_kit::rgb(0x419EDA),
        "zookeeper" => gpui_kit::rgb(0xF5A623),
        "consul" => gpui_kit::rgb(0xE03875),
        "databend" => gpui_kit::rgb(0x00A3FF),
        "starrocks" => gpui_kit::rgb(0x00B5AD),
        "redshift" => gpui_kit::rgb(0x8C4FFF),
        "snowflake" => gpui_kit::rgb(0x29B5E8),
        "trino" => gpui_kit::rgb(0xDD00A1),
        "bigquery" => gpui_kit::rgb(0x4285F4),
        "databricks" => gpui_kit::rgb(0xFF3621),
        "cloudflare-d1" => gpui_kit::rgb(0xF38020),
        _ => gpui_kit::rgb(0x5B8DEF),
    }
    .into()
}

pub(crate) fn connection_icon(key: &str, active: bool) -> Icon {
    let color = if active {
        connection_color(key)
    } else {
        gpui_kit::rgb(0x8C8C91).into()
    };
    let bytes: &[u8] = match key {
        "mysql" | "custom_mysql" => db_icon!("mysql"),
        "mariadb" => db_icon!("mariadb"),
        "tidb" => db_icon!("tidb"),
        "oceanbase" => db_icon!("oceanbase"),
        "postgres" | "custom_postgres" => db_icon!("postgres"),
        "redis" => db_icon!("redis"),
        "sqlite" => db_icon!("sqlite"),
        "sqlserver" => db_icon!("sqlserver"),
        "oracle" => db_icon!("oracle"),
        "mongodb" => db_icon!("mongodb"),
        "clickhouse" => db_icon!("clickhouse"),
        "firebird" => db_icon!("firebird"),
        "cockroachdb" => db_icon!("cockroachdb"),
        "db2" => db_icon!("db2"),
        "informix" => db_icon!("informix"),
        "jdbcx" => db_icon!("jdbcx"),
        "spanner" => db_icon!("spanner"),
        "iris" | "cache" => db_icon!("iris"),
        "dolt" => db_icon!("dolt"),
        "duckdb" => db_icon!("duckdb"),
        "dm" => db_icon!("dm"),
        "kingbase" => db_icon!("kingbase"),
        "opengauss" => db_icon!("opengauss"),
        "h2" => db_icon!("h2"),
        "elasticsearch" => db_icon!("elasticsearch"),
        "easysearch" => db_icon!("easysearch"),
        "solr" => db_icon!("solr"),
        "couchdb" => db_icon!("couchdb"),
        "cassandra" => db_icon!("cassandra"),
        "neo4j" => db_icon!("neo4j"),
        "qdrant" => db_icon!("qdrant"),
        "weaviate" => db_icon!("weaviate"),
        "chromadb" => db_icon!("chromadb"),
        "influxdb" => db_icon!("influxdb"),
        "tdengine" => db_icon!("tdengine"),
        "questdb" => db_icon!("questdb"),
        "iotdb" => db_icon!("iotdb"),
        "kafka" => db_icon!("kafka"),
        "rocketmq" => db_icon!("rocketmq"),
        "rabbitmq" => db_icon!("rabbitmq"),
        "mqtt" => db_icon!("mqtt"),
        "etcd" => db_icon!("etcd"),
        "zookeeper" => db_icon!("zookeeper"),
        "consul" => db_icon!("consul"),
        "databend" => db_icon!("databend"),
        "starrocks" => db_icon!("starrocks"),
        "redshift" => db_icon!("redshift"),
        "snowflake" => db_icon!("snowflake"),
        "trino" => db_icon!("trino"),
        "bigquery" => db_icon!("bigquery"),
        "databricks" => db_icon!("databricks"),
        "cloudflare-d1" => db_icon!("cloudflare-d1"),
        _ => {
            return Icon::new(AssetIcon::Database)
                .with_size(px(16.))
                .text_color(color);
        }
    };
    Icon::default().data(bytes).with_size(px(16.)).text_color(color)
}

/// Rows used as popover triggers only need `Selectable`; the row itself
/// carries all of the visual state.
pub(crate) struct MenuTrigger(pub(crate) gpui_kit::Stateful<gpui_kit::Div>);

impl Selectable for MenuTrigger {
    fn selected(self, _: bool) -> Self {
        self
    }

    fn is_selected(&self) -> bool {
        false
    }
}

impl IntoElement for MenuTrigger {
    type Element = <gpui_kit::Stateful<gpui_kit::Div> as IntoElement>::Element;

    fn into_element(self) -> Self::Element {
        self.0.into_element()
    }
}

pub(crate) fn query_menu_row<F>(
    id: String,
    label: String,
    selected: bool,
    icon: Option<AssetIcon>,
    colors: gpui_kit::component::ThemeColor,
    on_click: F,
) -> gpui_kit::Stateful<gpui_kit::Div>
where
    F: Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
{
    let muted = colors.muted_foreground;
    h_flex()
        .id(SharedString::from(id))
        .h(px(30.))
        .px_2()
        .gap_2()
        .items_center()
        .rounded(px(5.))
        .cursor_pointer()
        .when(selected, |this| this.bg(colors.secondary))
        .hover(|style| style.bg(colors.sidebar_accent))
        .when_some(icon, |this, icon| {
            this.child(Icon::new(icon).with_size(px(15.)).text_color(muted))
        })
        .child(
            div()
                .flex_1()
                .text_size(px(13.))
                .font_weight(if selected {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .child(label),
        )
        .when(selected, |this| {
            this.child(
                Icon::new(AssetIcon::Check)
                    .with_size(px(14.))
                    .text_color(colors.primary),
            )
        })
        .on_click(on_click)
}

pub(crate) fn query_menu_caption(
    text: &'static str,
    colors: gpui_kit::component::ThemeColor,
) -> impl IntoElement {
    div()
        .px_2()
        .pt_1()
        .pb(px(2.))
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(colors.muted_foreground)
        .child(text)
}

pub(crate) fn tool_menu_row(
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
pub(crate) fn tree_row(
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
    /// Sidebar glyph: the filled left panel means "expand", the divider means
    /// "collapse", matching the states the button switches between.
    pub(crate) fn sidebar_glyph(open: bool, color: Hsla) -> impl IntoElement {
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

    pub(crate) fn props_glyph(open: bool, color: Hsla) -> impl IntoElement {
        let frame = div()
            .w(px(14.))
            .h(px(14.))
            .flex_none()
            .rounded(px(2.))
            .border_1()
            .border_color(color);
        if open {
            frame.child(div().h_full().w(px(1.)).ml(px(8.)).bg(color))
        } else {
            frame.child(div().h_full().w(px(4.)).ml(px(8.)).bg(color))
        }
    }

    pub(crate) fn sidebar_toggle(&self, cx: &mut Context<Self>) -> gpui_kit::Stateful<gpui_kit::Div> {
        let colors = cx.theme().colors;
        ui::button::icon_button(
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
    pub(crate) fn content_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .pl(px(0.))
            .pr(px(8.))
            .pb(px(8.))
            // no tab strip: keep the same 8px gap at the top as the sides
            .when(self.tabs.is_empty() && self.queries.is_empty(), |this| {
                this.pt(px(8.))
            })
            .child({
                let bounds = self.content_bounds.clone();
                let split = self.props_split * self.props_anim;
                let main_grow = if self.props_anim > 0.0 {
                    1.0 - split
                } else {
                    1.0
                };
                h_flex()
                    .relative()
                    .size_full()
                    .min_h_0()
                    .on_prepaint(move |value, _, _| bounds.set(value))
                    .child(
                        v_flex()
                            .flex_basis(gpui_kit::relative(0.))
                            .flex_grow(main_grow)
                            .h_full()
                            .min_h_0()
                            .min_w_0()
                            .rounded(px(10.))
                            .overflow_hidden()
                            .bg(colors.background)
                            .border_1()
                            .border_color(colors.border)
                            .child(self.main_pane(cx)),
                    )
                    .when(self.props_anim > 0.0, |this| {
                        this.child(
                            div()
                                .id("content-props-split")
                                .w(px(9. * self.props_anim))
                                .h_full()
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_col_resize()
                                .occlude()
                                .on_drag(PropsDividerDrag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(
                                    |this,
                                     e: &gpui_kit::DragMoveEvent<PropsDividerDrag>,
                                     _,
                                     cx| {
                                        let bounds = this.content_bounds.get();
                                        if bounds.size.width > px(0.) {
                                            let available =
                                                (bounds.size.width - px(9.)).max(px(1.));
                                            let ratio = ((bounds.origin.x
                                                + bounds.size.width
                                                - e.event.position.x)
                                                / available)
                                                .clamp(0.2, 0.5);
                                            this.props_split = ratio;
                                            cx.notify();
                                        }
                                    },
                                )),
                        )
                        .child(
                            v_flex()
                                .flex_basis(gpui_kit::relative(0.))
                                .flex_grow(split)
                                .h_full()
                                .min_h_0()
                                .min_w_0()
                                .opacity(self.props_anim)
                                .rounded(px(10.))
                                .overflow_hidden()
                                .bg(colors.background)
                                .border_1()
                                .border_color(colors.border)
                                .child(self.table_props_panel(cx)),
                        )
                    })
            })
    }
}
