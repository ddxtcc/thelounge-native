use super::*;

impl LoungeApp {
    pub(super) fn input_for_role(&self, role: InputRole) -> Option<&Entity<TextInput>> {
        match role {
            InputRole::Composer => Some(&self.inputs.composer),
            InputRole::LoginServer => Some(&self.inputs.login_server),
            InputRole::LoginUser => Some(&self.inputs.login_user),
            InputRole::LoginPassword => Some(&self.inputs.login_password),
            InputRole::NetworkName => Some(&self.inputs.network_name),
            InputRole::NetworkHost => Some(&self.inputs.network_host),
            InputRole::NetworkPort => Some(&self.inputs.network_port),
            InputRole::NetworkNick => Some(&self.inputs.network_nick),
            InputRole::NetworkUsername => Some(&self.inputs.network_username),
            InputRole::NetworkRealname => Some(&self.inputs.network_realname),
            InputRole::NetworkPassword => Some(&self.inputs.network_password),
            InputRole::Search => Some(&self.inputs.search),
            InputRole::SettingHighlights => Some(&self.inputs.setting_highlights),
            InputRole::SettingHighlightExceptions => {
                Some(&self.inputs.setting_highlight_exceptions)
            }
            InputRole::SettingAwayMessage => Some(&self.inputs.setting_away_message),
            InputRole::EmojiSearch => Some(&self.inputs.emoji_search),
            InputRole::FontSearch => Some(&self.inputs.font_search),
            InputRole::PasswordOld => Some(&self.inputs.password_old),
            InputRole::PasswordNew => Some(&self.inputs.password_new),
            InputRole::PasswordVerify => Some(&self.inputs.password_verify),
        }
    }

    pub(super) fn render_login(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let on_accent = theme.on_accent;
        let mut card = div()
            .w(px(420.))
            .p(px(28.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .rounded(px(10.))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(26.))
                            .font_weight(FontWeight::BOLD)
                            .child("Lounge Native"),
                    )
                    .child(
                        div()
                            .text_color(theme.muted)
                            .text_size(px(13.))
                            .child("A native client for The Lounge"),
                    ),
            )
            .child(self.labeled_input("Server", &self.inputs.login_server))
            .child(self.labeled_input("Username (optional)", &self.inputs.login_user))
            .child(self.labeled_input("Password", &self.inputs.login_password));

        if let Some(error) = &self.login_error {
            card = card.child(
                div()
                    .text_color(theme.error)
                    .text_size(px(13.))
                    .child(error.clone()),
            );
        }

        card = card.child(
            div()
                .id("connect-button")
                .mt_2()
                .h(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(theme.accent)
                .text_color(on_accent)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|style| style.opacity(0.9))
                .on_click(cx.listener(|this, _, _window, cx| this.attempt_login(cx)))
                .child("Connect"),
        );

        card = card.child(
            div()
                .text_color(theme.muted)
                .text_size(px(12.))
                .child("Leave username and password empty to connect to a public server."),
        );

        if let ConnectionStatus::Reconnecting(reason) | ConnectionStatus::Failed(reason) =
            &self.status
        {
            card = card.child(
                div()
                    .text_color(theme.warning)
                    .text_size(px(12.))
                    .child(reason.clone()),
            );
        }

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(card)
            .into_any_element()
    }

    pub(super) fn labeled_input(&self, label: &str, input: &Entity<TextInput>) -> AnyElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted)
                    .child(label.to_string()),
            )
            .child(input.clone())
            .into_any_element()
    }

    pub(super) fn render_chat(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let show_users = self.config.show_user_list
            && self
                .state
                .active_channel()
                .map(|(_, channel)| channel.is_channel())
                .unwrap_or(false);
        let special_channel = self
            .state
            .active_channel()
            .map(|(_, channel)| channel.chan.special.is_some())
            .unwrap_or(false);

        let mut root = div().size_full().flex();
        if self.config.show_sidebar {
            root = root.child(self.render_sidebar(window, cx));
        }
        root = root.child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .child(self.render_header(window, cx))
                .child(self.render_messages(window, cx))
                .when(!special_channel, |this| {
                    this.child(self.render_composer(window, cx))
                }),
        );
        if show_users {
            root = root.child(self.render_users(window, cx));
        }
        root.into_any_element()
    }

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
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.open_channel_menu(
                                        ChannelMenu {
                                            chan_id: id,
                                            network_uuid: network_uuid.clone(),
                                            name: name.clone(),
                                            is_query,
                                            muted,
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
                            .flex_col()
                            .child(div().font_weight(FontWeight::BOLD).child("Lounge Native"))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if online { theme.success } else { theme.warning })
                                    .truncate()
                                    .child(status_text),
                            ),
                    )
                    .child(
                        div()
                            .id("add-network")
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
                            .on_click(cx.listener(|this, _, _, cx| this.open_network_modal(cx))),
                    ),
            )
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(list),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(theme.border)
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.sidebar_button("Network actions", cx, |this, _, cx| {
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
                    }))
                    .child(self.sidebar_button("Join channel", cx, |this, window, cx| {
                        this.inputs
                            .composer
                            .update(cx, |input, cx| input.set_content("/join ", cx));
                        this.focus_composer(window, cx);
                        cx.notify();
                    }))
                    .child(
                        self.sidebar_button("Mentions", cx, |this, _, cx| this.open_mentions(cx)),
                    )
                    .child(
                        self.sidebar_button("Settings", cx, |this, _, cx| this.open_settings(cx)),
                    )
                    .child(self.sidebar_button("Sign out", cx, |this, _, cx| this.sign_out(cx))),
            )
            .into_any_element()
    }

    pub(super) fn sidebar_button(
        &self,
        label: &str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .id(ElementId::Name(format!("side-{label}").into()))
            .px_2()
            .py(px(6.))
            .rounded(px(5.))
            .text_size(px(13.))
            .text_color(theme.muted)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover).text_color(theme.text))
            .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
            .child(label.to_string())
            .into_any_element()
    }

    pub(super) fn render_header(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let active = self.state.active_channel();
        let Some((network, channel)) = active else {
            return div()
                .h(px(52.))
                .px_4()
                .flex()
                .items_center()
                .gap_3()
                .border_b_1()
                .border_color(theme.border)
                .child(self.sidebar_toggle(cx))
                .child(
                    div()
                        .text_color(theme.muted)
                        .child("Select a network or add one to begin."),
                )
                .into_any_element();
        };

        let name = if channel.is_lobby() {
            network.name.clone()
        } else {
            channel.chan.name.clone()
        };
        let topic = channel.chan.topic.clone();
        let user_count = channel.users.len();
        let users_shown = self.config.show_user_list && channel.is_channel();
        let network_name = network.name.clone();

        div()
            .h(px(52.))
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .border_b_1()
            .border_color(theme.border)
            .child(self.sidebar_toggle(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(name))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.muted)
                                    .child(network_name),
                            ),
                    )
                    .child(
                        div()
                            .id("topic")
                            .text_size(px(12.))
                            .text_color(theme.muted)
                            .truncate()
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.inputs
                                    .composer
                                    .update(cx, |input, cx| input.set_content("/topic ", cx));
                                this.focus_composer(window, cx);
                                cx.notify();
                            }))
                            .child(if topic.is_empty() {
                                "Set a topic...".to_string()
                            } else {
                                topic
                            }),
                    ),
            )
            .when(channel.is_channel(), |this| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted)
                        .child(format!(
                            "{user_count} {}",
                            if user_count == 1 { "user" } else { "users" }
                        )),
                )
            })
            .when(channel.is_channel() || channel.is_query(), |this| {
                let muted = channel.chan.muted;
                this.child(self.header_button(
                    if muted { "Unmute" } else { "Mute" },
                    muted,
                    cx,
                    move |this, _, cx| {
                        if let (Some(conn), Some(target)) = (&this.conn, this.state.active) {
                            conn.emit(
                                "mute:change",
                                Some(serde_json::json!({"target": target, "setMutedTo": !muted})),
                            );
                        }
                        cx.notify();
                    },
                ))
            })
            .when(channel.is_channel() || channel.is_query(), |this| {
                let is_query = channel.is_query();
                this.child(self.header_button(
                    if is_query { "Close" } else { "Leave" },
                    false,
                    cx,
                    move |this, _, cx| {
                        if let (Some(conn), Some(target)) = (&this.conn, this.state.active) {
                            conn.input(target, if is_query { "/close" } else { "/part" }, None);
                        }
                        cx.notify();
                    },
                ))
            })
            .child(self.header_button("Users", users_shown, cx, |this, _, cx| {
                this.config.show_user_list = !this.config.show_user_list;
                this.config.save();
                cx.notify();
            }))
            .child(
                self.header_button("Search", self.search_open, cx, |this, _, cx| {
                    this.open_search(cx)
                }),
            )
            .into_any_element()
    }

    pub(super) fn sidebar_toggle(&self, cx: &mut Context<Self>) -> AnyElement {
        self.header_button("☰", !self.config.show_sidebar, cx, |this, _, cx| {
            this.config.show_sidebar = !this.config.show_sidebar;
            this.config.save();
            cx.notify();
        })
    }

    pub(super) fn header_button(
        &self,
        label: &str,
        active: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .id(ElementId::Name(format!("head-{label}").into()))
            .px_2()
            .py(px(4.))
            .rounded(px(4.))
            .text_size(px(12.))
            .cursor_pointer()
            .when(active, |this| {
                this.bg(theme.accent_dim).text_color(theme.accent)
            })
            .when(!active, |this| {
                this.text_color(theme.muted)
                    .hover(|style| style.bg(theme.hover))
            })
            .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
            .child(label.to_string())
            .into_any_element()
    }

    pub(super) fn render_composer(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let typing = self
            .state
            .active_channel()
            .map(|(_, channel)| {
                channel
                    .typing
                    .iter()
                    .filter(|typing| typing.updated.elapsed() < Duration::from_secs(10))
                    .map(|typing| typing.nick.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut footer = div()
            .border_t_1()
            .border_color(theme.border)
            .px_3()
            .pt_2()
            .pb_3()
            .flex()
            .flex_col()
            .gap_2();

        if !typing.is_empty() {
            let label = if typing.len() == 1 {
                format!("{} is typing...", typing[0])
            } else {
                format!("{} are typing...", typing.join(", "))
            };
            footer = footer.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child(label),
            );
        }

        if let Some(reply) = &self.reply_to {
            footer = footer.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .bg(theme.panel)
                    .text_size(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_color(theme.muted)
                            .child(format!("Replying to {}: {}", reply.nick, reply.text)),
                    )
                    .child(
                        div()
                            .id("cancel-reply")
                            .px_1()
                            .cursor_pointer()
                            .text_color(theme.muted)
                            .hover(|style| style.text_color(theme.error))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.reply_to = None;
                                cx.notify();
                            }))
                            .child("x"),
                    ),
            );
        }

        let upload_enabled = self
            .state
            .configuration
            .as_ref()
            .map(|config| config.file_upload)
            .unwrap_or(false);
        let emoji_button = div()
            .id("emoji-picker-button")
            .h(px(34.))
            .px_2()
            .flex()
            .items_center()
            .rounded(px(6.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .text_size(px(12.))
            .text_color(theme.muted)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover).text_color(theme.text))
            .on_click(cx.listener(|this, _, _, cx| this.open_emoji_picker(cx)))
            .child("Emoji");
        let upload_button = upload_enabled.then(|| {
            div()
                .id("upload-file")
                .h(px(34.))
                .px_2()
                .flex()
                .items_center()
                .rounded(px(6.))
                .bg(theme.panel)
                .border_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .text_color(theme.text)
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover).text_color(theme.text))
                .on_click(cx.listener(|this, _, _, cx| this.prompt_for_upload(cx)))
                .child("Upload")
        });

        footer
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(self.inputs.composer.clone()),
                    )
                    .child(emoji_button)
                    .children(upload_button),
            )
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
        let mut list = div().flex().flex_col().gap_0p5().p_2();
        for user in &channel.users {
            let nick = user.nick.clone();
            let mode = user_mode_prefix(user, network);
            let color = if self.config.colored_nicks {
                rgb(format::nick_color(&user.nick)).into()
            } else {
                theme.text
            };
            let away = !user.away.is_empty()
                || channel.chan.user_away.as_deref() == Some(user.nick.as_str());
            list = list.child(
                div()
                    .id(ElementId::Name(format!("user-{nick}").into()))
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
                            move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.open_user_menu(&nick, &network_uuid, chan_id, cx);
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

fn user_mode_prefix(user: &User, network: &NetworkState) -> Option<String> {
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
