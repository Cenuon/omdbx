use crate::*;

#[derive(Clone)]
pub(crate) struct ConnectionMenuState {
    pub connection: Connection,
    pub folders: Vec<SidebarFolder>,
    pub stars: Vec<String>,
    pub profiles: Vec<ConnectionProfile>,
    pub active_profile: Option<String>,
    pub show_empty_databases: bool,
}

const MENU_COLORS: [(&str, &str); 8] = [
    ("红", "#E5534B"),
    ("橙", "#E08C3C"),
    ("黄", "#D9B23C"),
    ("绿", "#4F9D69"),
    ("蓝", "#3F7FD0"),
    ("紫", "#8A6BD1"),
    ("灰", "#8A8A84"),
    ("彩虹", "rainbow"),
];

fn rainbow_swatch() -> impl IntoElement {
    h_flex()
        .w(px(12.))
        .h(px(12.))
        .gap(px(1.))
        .items_center()
        .children(
            [
                "#e5534b", "#e08c3c", "#d9b23c", "#4f9d69", "#3f7fd0", "#8a6bd1",
            ]
            .into_iter()
            .map(|color| {
                let color = connection_color_value(color).unwrap_or_else(|| {
                    gpui_kit::rgb(0x8A8A84).into()
                });
                div().flex_1().h_full().rounded(px(1.)).bg(color)
            }),
        )
}

fn color_swatch(value: &str) -> gpui_kit::AnyElement {
    if value == "rainbow" {
        return rainbow_swatch().into_any_element();
    }
    let color = connection_color_value(value).unwrap_or_else(|| gpui_kit::rgb(0x8A8A84).into());
    div()
        .w(px(12.))
        .h(px(12.))
        .rounded_full()
        .bg(color)
        .into_any_element()
}

pub(crate) fn connection_context_menu(
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    app: gpui_kit::WeakEntity<Omdbx>,
    state: ConnectionMenuState,
) -> PopupMenu {
    let connection_id = state.connection.id.clone();
    let starred = state.stars.iter().any(|id| id == &connection_id);
    let color = state.connection.color().map(str::to_string);
    let mut menu = app_menu(menu, cx).min_w(px(232.)).max_w(px(320.));

    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("打开连接")
            .icon(Icon::new(AssetIcon::Power))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| this.open_connection(&id, cx));
            }),
    );
    menu = menu.separator();

    let profile_app = app.clone();
    let profile_connection = connection_id.clone();
    let profiles = state.profiles.clone();
    let active_profile = state.active_profile.clone();
    menu = menu.submenu_with_icon(
        Some(Icon::new(AssetIcon::Settings2)),
        "切换连接配置文件",
        window,
        cx,
        move |menu, _, cx| {
            let mut menu = app_menu(menu, cx).min_w(px(224.));
            if profiles.is_empty() {
                menu = menu.item(PopupMenuItem::new("暂无已保存的配置文件").disabled(true));
            }
            for profile in &profiles {
                let weak = profile_app.clone();
                let connection_id = profile_connection.clone();
                let profile_id = profile.id.clone();
                let selected = active_profile.as_deref() == Some(profile.id.as_str());
                menu = menu.item(
                    PopupMenuItem::new(profile.name.clone())
                        .checked(selected)
                        .on_click(move |_, _, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                this.switch_connection_profile(
                                    &connection_id,
                                    &profile_id,
                                    cx,
                                )
                            });
                        }),
                );
            }
            menu = menu.separator();
            let weak = profile_app.clone();
            let connection_id = profile_connection.clone();
            menu = menu.item(
                PopupMenuItem::new("保存当前配置为文件…")
                    .icon(Icon::new(AssetIcon::Plus))
                    .on_click(move |_, window, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.open_new_profile_prompt(&connection_id, window, cx)
                        });
                    }),
            );
            if let Some(active) = active_profile.clone() {
                let weak = profile_app.clone();
                let connection_id = profile_connection.clone();
                menu = menu.item(
                    PopupMenuItem::new("删除当前配置文件")
                        .icon(Icon::new(AssetIcon::Trash))
                        .on_click(move |_, _, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                if this.active_profiles.get(&connection_id) == Some(&active) {
                                    this.delete_active_profile(&connection_id, cx);
                                }
                            });
                        }),
                );
            }
            menu
        },
    );

    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("编辑连接…")
            .icon(Icon::new(AssetIcon::SquarePen))
            .on_click(move |_, window, cx| {
                let _ = weak.update(cx, |this, cx| {
                    this.open_edit_connection_dialog(&id, window, cx)
                });
            }),
    );
    let weak = app.clone();
    menu = menu.item(
        PopupMenuItem::new("新建连接")
            .icon(Icon::new(AssetIcon::Plus))
            .on_click(move |_, window, cx| {
                let _ = weak.update(cx, |this, cx| this.open_connection_dialog(window, cx));
            }),
    );
    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("删除连接")
            .icon(Icon::new(AssetIcon::Trash))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| this.confirm_delete_connection(&id, cx));
            }),
    );
    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("复制连接…")
            .icon(Icon::new(AssetIcon::Copy))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| this.duplicate_connection(&id, cx));
            }),
    );
    menu = menu.separator();

    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("新建查询")
            .icon(Icon::new(AssetIcon::SquareTerminal))
            .action(Box::new(NewQuery))
            .on_click(move |_, window, cx| {
                let _ = weak.update(cx, |this, cx| {
                    this.open_query_for_connection(&id, window, cx)
                });
            }),
    );
    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("命令列界面")
            .icon(Icon::new(AssetIcon::Command))
            .on_click(move |_, window, cx| {
                let _ = weak.update(cx, |this, cx| this.open_command_line(&id, window, cx));
            }),
    );
    menu = menu
        .item(
            PopupMenuItem::new("命令监控")
                .icon(Icon::new(AssetIcon::Activity))
                .disabled(true),
        )
        .item(
            PopupMenuItem::new("运行命令文件…")
                .icon(Icon::new(AssetIcon::FilePlay))
                .disabled(true),
        )
        .separator();

    let weak = app.clone();
    menu = menu.item(
        PopupMenuItem::new("显示空数据库")
            .icon(Icon::new(AssetIcon::Eye))
            .checked(state.show_empty_databases)
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| {
                    let show = !this.show_empty_databases;
                    this.toggle_show_empty_databases(show, cx);
                });
            }),
    );
    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new(if starred { "取消星标" } else { "添加星标" })
            .icon(Icon::new(if starred {
                AssetIcon::StarOff
            } else {
                AssetIcon::Star
            }))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| this.toggle_star(&id, cx));
            }),
    );

    let color_app = app.clone();
    let color_connection = connection_id.clone();
    let selected_color = color.clone();
    menu = menu.submenu_with_icon(
        Some(Icon::new(AssetIcon::Palette)),
        "颜色",
        window,
        cx,
        move |menu, _, cx| {
            let mut menu = app_menu(menu, cx).min_w(px(160.));
            for (label, value) in MENU_COLORS {
                let weak = color_app.clone();
                let connection_id = color_connection.clone();
                let selected = selected_color.as_deref() == Some(value);
                menu = menu.item(
                    PopupMenuItem::element(move |_, _| {
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(color_swatch(value))
                            .child(label)
                    })
                    .checked(selected)
                    .on_click(move |_, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.set_connection_color(&connection_id, Some(value), cx)
                        });
                    }),
                );
            }
            menu = menu.separator();
            let weak = color_app.clone();
            let connection_id = color_connection.clone();
            menu = menu.item(
                PopupMenuItem::new("清除颜色")
                    .checked(selected_color.is_none())
                    .on_click(move |_, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.set_connection_color(&connection_id, None, cx)
                        });
                    }),
            );
            menu
        },
    );

    let group_app = app.clone();
    let group_connection = connection_id.clone();
    let folders = state.folders.clone();
    let current_folder = folders
        .iter()
        .find(|folder| folder.connections.contains(&connection_id))
        .map(|folder| folder.id.clone());
    menu = menu.submenu_with_icon(
        Some(Icon::new(AssetIcon::Folder)),
        "管理组",
        window,
        cx,
        move |menu, _, cx| {
            let mut menu = app_menu(menu, cx).min_w(px(210.));
            let weak = group_app.clone();
            menu = menu.item(
                PopupMenuItem::new("新建组…")
                    .icon(Icon::new(AssetIcon::FolderPlus))
                    .on_click(move |_, window, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.open_new_folder_prompt(window, cx)
                        });
                    }),
            );
            if !folders.is_empty() {
                menu = menu.separator();
                for folder in &folders {
                    let weak = group_app.clone();
                    let connection_id = group_connection.clone();
                    let folder_id = folder.id.clone();
                    let selected = current_folder.as_deref() == Some(folder.id.as_str());
                    menu = menu.item(
                        PopupMenuItem::new(format!("移入「{}」", folder.name))
                            .checked(selected)
                            .on_click(move |_, _, cx| {
                                let _ = weak.update(cx, |this, cx| {
                                    this.move_connection_to_folder(
                                        &connection_id,
                                        Some(&folder_id),
                                        cx,
                                    )
                                });
                            }),
                    );
                }
            }
            if let Some(folder_id) = current_folder.clone() {
                menu = menu.separator();
                let weak = group_app.clone();
                let connection_id = group_connection.clone();
                menu = menu.item(
                    PopupMenuItem::new("移出分组").on_click(move |_, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.move_connection_to_folder(&connection_id, None, cx)
                        });
                    }),
                );
                let weak = group_app.clone();
                let id = folder_id.clone();
                menu = menu.item(
                    PopupMenuItem::new("重命名组…").on_click(move |_, window, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.open_rename_folder_prompt(&id, window, cx)
                        });
                    }),
                );
                let weak = group_app.clone();
                let id = folder_id;
                menu = menu.item(
                    PopupMenuItem::new("删除组…").on_click(move |_, _, cx| {
                        let _ = weak.update(cx, |this, cx| this.confirm_delete_folder(&id, cx));
                    }),
                );
            }
            menu
        },
    );

    menu = menu.separator();
    let weak = app.clone();
    let id = connection_id.clone();
    menu = menu.item(
        PopupMenuItem::new("共享…")
            .icon(Icon::new(AssetIcon::Share2))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| this.share_connection(&id, cx));
            }),
    );
    let weak = app;
    let id = connection_id;
    menu.item(
        PopupMenuItem::new("刷新")
            .icon(Icon::new(AssetIcon::RefreshCw))
            .action(Box::new(RefreshConnections))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |this, cx| this.refresh_connection(&id, cx));
            }),
    )
}
