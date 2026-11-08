use super::tooltip::Tooltip;
use super::*;

#[derive(Clone, Copy)]
enum SidebarAction {
    Network,
    Join,
    Mentions,
    Settings,
    SignOut,
}

impl LoungeApp {
    pub(super) fn render_sidebar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let on_accent = theme.on_accent;
        let mut list = div().flex().flex_col().gap_1().p_2().w_full();

        for network in &self.state.networks {
            let uuid = network.uuid.clone();
            let collapsed = self.collapsed_networks.contains(&uuid);
            let unread: u32 = network
                .channels
                .iter()
                .map(|channel| channel.chan.unread + channel.chan.highlight)
                .sum();
            let uuid_click = uuid.clone();
            let network_label = network.name.clone();
            let drag = SortDrag {
                kind: SortKind::Network,
                network_uuid: uuid.clone(),
                channel_id: None,
                label: network.name.clone(),
            };
            let drop_uuid = uuid.clone();
            list = list.child(
                div()
                    .id(ElementId::Name(format!("net-{uuid}").into()))
                    .tab_index(0)
                    .tab_stop(!self.last_overlay_open)
                    .focus(|style| style.bg(theme.border).text_color(theme.text))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1p5()
                    .rounded(px(5.))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_drag(drag, move |drag: &SortDrag, position, _, cx| {
                        LoungeApp::drag_preview(drag, position, theme, cx)
                    })
                    .on_drop(cx.listener(move |this, drag: &SortDrag, _, cx| {
                        this.drop_sort(drag, SortTarget::Network(drop_uuid.clone()), cx)
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.collapsed_networks.remove(&uuid_click) {
                            this.collapsed_networks.insert(uuid_click.clone());
                        }
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener({
                            let uuid = uuid.clone();
                            move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.open_network_menu(&uuid, cx);
                            }
                        }),
                    )
                    .child(
                        div()
                            .size(px(7.))
                            .rounded_full()
                            .bg(if network.status.connected {
                                theme.success
                            } else {
                                theme.faint
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(network_label),
                    )
                    .when(unread > 0, |this| {
                        this.child(unread_badge(unread, false, theme))
                    })
                    .child(
                        div()
                            .id(ElementId::Name(format!("net-menu-{uuid}").into()))
                            .tab_index(0)
                            .tab_stop(!self.last_overlay_open)
                            .focus(|style| style.bg(theme.border).text_color(theme.text))
                            .w(px(20.))
                            .h(px(18.))
                            .text_center()
                            .text_color(theme.faint)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.open_network_menu(&uuid, cx);
                            }))
                            .tooltip(move |_, cx| {
                                cx.new(|_| Tooltip::new("Network actions", theme)).into()
                            })
                            .child("..."),
                    ),
            );

            if collapsed {
                continue;
            }

            for channel in &network.channels {
                let id = channel.chan.id;
                let selected = self.state.active == Some(id);
                let name = if channel.is_lobby() {
                    network.name.clone()
                } else {
                    channel.chan.name.clone()
                };
                let icon = match channel.chan.kind {
                    lounge_core::ChanType::Query => "@",
                    lounge_core::ChanType::Channel => "#",
                    lounge_core::ChanType::Special => "=",
                    _ => "*",
                };
                let highlight = channel.chan.highlight;
                let unread = channel.chan.unread;
                let muted = channel.chan.muted;
                let sortable = matches!(
                    channel.chan.kind,
                    lounge_core::ChanType::Channel | lounge_core::ChanType::Query
                );
                let network_uuid = network.uuid.clone();
                let drag = SortDrag {
                    kind: SortKind::Channel,
                    network_uuid: network_uuid.clone(),
                    channel_id: Some(id),
                    label: name.clone(),
                };
                let drop_network = network_uuid.clone();
                list = list.child(
                    div()
                        .id(ElementId::Name(format!("chan-{id}").into()))
                        .tab_index(0)
                        .tab_stop(!self.last_overlay_open)
                        .focus(|style| style.bg(theme.border).text_color(theme.text))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .pl(px(14.))
                        .pr_2()
                        .py(px(5.))
                        .rounded(px(5.))
                        .cursor_pointer()
                        .when(selected, |this| this.bg(theme.accent_dim))
                        .when(!selected, |this| this.hover(|style| style.bg(theme.hover)))
                        .when(sortable, |this| {
                            this.on_drag(drag, move |drag: &SortDrag, position, _, cx| {
                                LoungeApp::drag_preview(drag, position, theme, cx)
                            })
                            .on_drop(cx.listener(
                                move |this, drag: &SortDrag, _, cx| {
                                    this.drop_sort(
                                        drag,
                                        SortTarget::Channel {
                                            network_uuid: drop_network.clone(),
                                            channel_id: id,
                                        },
                                        cx,
                                    )
                                },
                            ))
                        })
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let name = name.clone();
                                let network_uuid = network_uuid.clone();
                                let is_query = channel.is_query();
                                let muted = channel.chan.muted;
                                move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.open_channel_menu(
                                        ChannelMenu {
                                            chan_id: id,
                                            network_uuid: network_uuid.clone(),
                                            name: name.clone(),
                                            is_query,
                                            muted,
                                            position: event.position,
                                        },
                                        cx,
                                    );
                                }
                            }),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.switch_channel(id, window, cx);
                        }))
                        .child(
                            div()
                                .w(px(12.))
                                .text_color(if selected { theme.accent } else { theme.muted })
                                .child(icon),
                        )
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .when(muted, |this| this.text_color(theme.faint))
                                .child(name),
                        )
                        .when(highlight > 0, |this| {
                            this.child(
                                div()
                                    .px(px(6.))
                                    .rounded_full()
                                    .bg(theme.error)
                                    .text_color(theme.on_accent)
                                    .text_size(px(11.))
                                    .child(highlight.to_string()),
                            )
                        })
                        .when(highlight == 0 && unread > 0, |this| {
                            this.child(
                                div()
                                    .px(px(6.))
                                    .rounded_full()
                                    .bg(theme.accent)
                                    .text_color(on_accent)
                                    .text_size(px(11.))
                                    .child(unread.to_string()),
                            )
                        }),
                );
            }
        }

        if self.state.networks.is_empty() {
            list = list.child(
                div()
                    .p_3()
                    .text_color(theme.muted)
                    .text_size(px(12.))
                    .child("No networks yet. Add one to get started."),
            );
        }

        let status_text = self.status.label().to_string();
        let online = self.status.is_online();

        div()
            .w(px(238.))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .h(px(52.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(self.narrow, |this| {
                                this.child(
                                    div()
                                        .id("close-sidebar")
                                        .size(px(24.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(5.))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.hover))
                                        .child("<")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.sidebar_overlay = false;
                                            cx.notify();
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div().font_weight(FontWeight::BOLD).child("Lounge Native"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(if online {
                                                theme.success
                                            } else {
                                                theme.warning
                                            })
                                            .truncate()
                                            .child(status_text),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("add-network")
                            .tab_index(0)
                            .tab_stop(!self.last_overlay_open)
                            .focus(|style| style.bg(theme.border).text_color(theme.text))
                            .size(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .bg(theme.panel)
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child("+")
                            .on_click(cx.listener(|this, _, _, cx| this.open_network_modal(cx)))
                            .tooltip(move |_, cx| {
                                cx.new(|_| Tooltip::new("Add network", theme)).into()
                            }),
                    ),
            )
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(list),
            )
            .child(self.sidebar_footer(cx))
            .into_any_element()
    }

    fn sidebar_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let icon_buttons = self.config.icon_buttons;
        let mut footer = div()
            .border_t_1()
            .border_color(theme.border)
            .p_2()
            .min_h(px(COMPOSER_MIN_HEIGHT));
        footer = if icon_buttons {
            footer.flex().items_center().justify_between()
        } else {
            footer.flex().flex_col().gap_1()
        };
        let entries = if icon_buttons {
            [
                ("⋮", "Network actions", SidebarAction::Network),
                ("+", "Join channel", SidebarAction::Join),
                ("@", "Recent mentions", SidebarAction::Mentions),
                ("⚙", "Settings", SidebarAction::Settings),
                ("⏻", "Sign out", SidebarAction::SignOut),
            ]
        } else {
            [
                ("Network actions", "Network actions", SidebarAction::Network),
                ("Join channel", "Join channel", SidebarAction::Join),
                ("Mentions", "Recent mentions", SidebarAction::Mentions),
                ("Settings", "Settings", SidebarAction::Settings),
                ("Sign out", "Sign out", SidebarAction::SignOut),
            ]
        };
        let unread_mentions = self.unread_mentions();
        for (label, tooltip, action) in entries {
            let label = if matches!(action, SidebarAction::Mentions) && unread_mentions > 0 {
                format!("{label} ({unread_mentions})")
            } else {
                label.to_string()
            };
            footer = footer.child(self.sidebar_button(
                &label,
                tooltip,
                cx,
                move |this, window, cx| match action {
                    SidebarAction::Network => {
                        let uuid = this
                            .state
                            .active_network()
                            .map(|network| network.uuid.clone())
                            .or_else(|| {
                                this.state
                                    .networks
                                    .first()
                                    .map(|network| network.uuid.clone())
                            });
                        match uuid {
                            Some(uuid) => this.open_network_menu(&uuid, cx),
                            None => this.open_network_modal(cx),
                        }
                    }
                    SidebarAction::Join => {
                        this.inputs
                            .composer
                            .update(cx, |input, cx| input.set_content("/join ", cx));
                        this.focus_composer(window, cx);
                        cx.notify();
                    }
                    SidebarAction::Mentions => this.open_mentions(cx),
                    SidebarAction::Settings => this.open_settings(cx),
                    SidebarAction::SignOut => this.sign_out(cx),
                },
            ));
        }
        footer.into_any_element()
    }

    pub(super) fn sidebar_button(
        &self,
        label: &str,
        tooltip: &str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let tooltip = tooltip.to_string();
        div()
            .id(ElementId::Name(format!("side-{label}").into()))
            .tab_index(0)
            .tab_stop(!self.last_overlay_open)
            .focus(|style| style.bg(theme.border).text_color(theme.text))
            .px_2()
            .py(px(6.))
            .rounded(px(5.))
            .text_size(px(13.))
            .text_color(theme.muted)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover).text_color(theme.text))
            .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new(tooltip.clone(), theme)).into())
            .child(label.to_string())
            .into_any_element()
    }

    pub(super) fn render_users(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some((network, channel)) = self.state.active_channel() else {
            return div().into_any_element();
        };
        if !channel.is_channel() {
            return div().into_any_element();
        }
        let filter = self.inputs.users_filter.read(cx).content().to_lowercase();
        let users: Vec<_> = channel
            .users
            .iter()
            .filter(|user| {
                filter.is_empty()
                    || user.nick.to_lowercase().contains(&filter)
                    || user.away.to_lowercase().contains(&filter)
                    || (user.is_bot && "bot".contains(&filter))
            })
            .collect();
        let mut list = div().flex().flex_col().gap_0p5().p_2();
        if users.is_empty() && !filter.is_empty() {
            list = list.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("No matching users."),
            );
        }
        for user in users {
            let nick = user.nick.clone();
            let mode = user_mode_prefix(user, network);
            let color = if self.config.colored_nicks {
                theme.nick_color(&user.nick)
            } else {
                theme.text
            };
            let away = !user.away.is_empty()
                || channel.chan.user_away.as_deref() == Some(user.nick.as_str());
            list = list.child(
                div()
                    .id(ElementId::Name(format!("user-{nick}").into()))
                    .tab_index(0)
                    .tab_stop(!self.last_overlay_open)
                    .focus(|style| style.bg(theme.border).text_color(theme.text))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .py(px(2.))
                    .rounded(px(3.))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(cx.listener({
                        let nick = nick.clone();
                        move |this, _, _, cx| this.insert_nick(&nick, cx)
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener({
                            let nick = nick.clone();
                            let network_uuid = network.uuid.clone();
                            let chan_id = channel.chan.id;
                            move |this, event: &gpui::MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.open_user_menu(
                                    &nick,
                                    &network_uuid,
                                    chan_id,
                                    event.position,
                                    cx,
                                );
                            }
                        }),
                    )
                    .child(
                        div()
                            .w(px(10.))
                            .text_size(px(11.))
                            .text_color(theme.warning)
                            .child(mode.unwrap_or_default()),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .when(away, |this| this.opacity(0.55))
                            .child(user.nick.clone()),
                    )
                    .when(user.is_bot, |this| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme.muted)
                                .child("bot"),
                        )
                    })
                    .when(away, |this| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme.muted)
                                .child("away"),
                        )
                    }),
            );
        }

        div()
            .w(px(210.))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.sidebar)
            .border_l_1()
            .border_color(theme.border)
            .child(
                div()
                    .h(px(52.))
                    .px_3()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .text_size(px(12.))
                    .text_color(theme.muted)
                    .child(format!("Users - {}", channel.users.len())),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(self.inputs.users_filter.clone()),
            )
            .child(
                div()
                    .id("users-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(list),
            )
            .into_any_element()
    }
}

fn unread_badge(count: u32, highlight: bool, theme: Theme) -> AnyElement {
    div()
        .px(px(6.))
        .rounded_full()
        .bg(if highlight { theme.error } else { theme.accent })
        .text_color(theme.on_accent)
        .text_size(px(11.))
        .child(count.to_string())
        .into_any_element()
}

pub(super) fn user_mode_prefix(user: &User, network: &NetworkState) -> Option<String> {
    let options = network.server_options.as_ref()?;
    for prefix in &options.prefix.prefix {
        if user.modes.contains(&prefix.mode) {
            return Some(prefix.symbol.clone());
        }
    }
    if !user.mode.is_empty() {
        return options.prefix.mode_to_symbol.get(&user.mode).cloned();
    }
    None
}
