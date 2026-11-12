use super::tooltip::Tooltip;
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
            InputRole::UsersFilter => Some(&self.inputs.users_filter),
            InputRole::SpecialFilter => Some(&self.inputs.special_filter),
            InputRole::Palette => Some(&self.inputs.palette),
            InputRole::PasswordOld => Some(&self.inputs.password_old),
            InputRole::PasswordNew => Some(&self.inputs.password_new),
            InputRole::PasswordVerify => Some(&self.inputs.password_verify),
            InputRole::ThemeName => Some(&self.inputs.theme_name),
            InputRole::ThemeHex => Some(&self.inputs.theme_hex),
        }
    }

    pub(super) fn text_input_focused(&self, window: &Window, cx: &gpui::App) -> bool {
        let Some(focused) = window.focused(cx) else {
            return false;
        };
        const ROLES: [InputRole; 25] = [
            InputRole::Composer,
            InputRole::LoginServer,
            InputRole::LoginUser,
            InputRole::LoginPassword,
            InputRole::NetworkName,
            InputRole::NetworkHost,
            InputRole::NetworkPort,
            InputRole::NetworkNick,
            InputRole::NetworkUsername,
            InputRole::NetworkRealname,
            InputRole::NetworkPassword,
            InputRole::Search,
            InputRole::SettingHighlights,
            InputRole::SettingHighlightExceptions,
            InputRole::SettingAwayMessage,
            InputRole::EmojiSearch,
            InputRole::FontSearch,
            InputRole::UsersFilter,
            InputRole::SpecialFilter,
            InputRole::Palette,
            InputRole::PasswordOld,
            InputRole::PasswordNew,
            InputRole::PasswordVerify,
            InputRole::ThemeName,
            InputRole::ThemeHex,
        ];
        ROLES.iter().any(|role| {
            self.input_for_role(*role)
                .is_some_and(|input| input.read(cx).focus_handle(cx) == focused)
        })
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
                .tab_index(0)
                .focus(|style| style.bg(theme.border).text_color(theme.text))
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
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                    if activation_key(event) {
                        this.attempt_login(cx);
                        cx.stop_propagation();
                    }
                }))
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
        self.narrow = window.viewport_size().width < px(900.);
        let channel_is_channel = self
            .state
            .active_channel()
            .map(|(_, channel)| channel.is_channel())
            .unwrap_or(false);
        let show_sidebar = if self.narrow {
            self.sidebar_overlay
        } else {
            self.config.show_sidebar
        };
        let show_users = if self.narrow {
            self.users_overlay
        } else {
            self.config.show_user_list
        } && channel_is_channel;
        let special_channel = self
            .state
            .active_channel()
            .map(|(_, channel)| channel.chan.special.is_some())
            .unwrap_or(false);

        let mut root = div().size_full().flex().relative();
        if !self.narrow && show_sidebar {
            root = root.child(self.render_sidebar(window, cx));
        }
        let layout_weak = cx.entity().downgrade();
        root = root.child(
            div()
                .flex_1()
                .h(window.window_bounds().get_bounds().size.height)
                .min_w(px(0.))
                .flex()
                .flex_col()
                .child(self.render_header(window, cx))
                .child(self.render_messages(window, cx))
                .when(!special_channel, |this| {
                    this.child(self.render_composer(window, cx))
                })
                .on_children_prepainted(move |bounds, window, cx| {
                    if bounds.len() < 3 {
                        return;
                    }
                    let available = (window.window_bounds().get_bounds().size.height
                        - bounds[0].size.height
                        - bounds[bounds.len() - 1].size.height)
                        .max(px(0.));
                    let _ = layout_weak.update(cx, |this, cx| {
                        if this.messages_available != available {
                            this.messages_available = available;
                            cx.notify();
                        }
                    });
                }),
        );
        if !self.narrow && show_users {
            root = root.child(self.render_users(window, cx));
        }
        if self.narrow && (show_sidebar || show_users) {
            root = root.child(
                div()
                    .id("narrow-backdrop")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .occlude()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sidebar_overlay = false;
                        this.users_overlay = false;
                        cx.stop_propagation();
                        cx.notify();
                    })),
            );
        }
        if self.narrow && show_sidebar {
            root = root.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .shadow_lg()
                    .child(self.render_sidebar(window, cx)),
            );
        }
        if self.narrow && show_users {
            root = root.child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .h_full()
                    .shadow_lg()
                    .child(self.render_users(window, cx)),
            );
        }
        root.into_any_element()
    }

    pub(super) fn render_header(
        &mut self,
        window: &mut Window,
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
        let compact_header = compact_header(
            window.viewport_size().width,
            self.narrow,
            self.config.show_sidebar,
            self.config.show_user_list,
            channel.is_channel(),
        );
        let icon_buttons = self.config.icon_buttons || compact_header;
        let topic = format::clean(&channel.chan.topic);
        let topic_editable = channel.is_channel();
        let topic_label = if topic_editable {
            if topic.is_empty() {
                "Set a topic...".to_string()
            } else {
                topic
            }
        } else {
            channel_secondary_label(channel)
        };
        let user_count = channel.users.len();
        let users_shown = if self.narrow {
            self.users_overlay
        } else {
            self.config.show_user_list
        } && channel.is_channel();
        let network_name = network.name.clone();
        let pin_count = self
            .config
            .pinned_messages
            .iter()
            .filter(|pin| pin.channel == channel.chan.id)
            .count();
        let pins_button = self.config.features.pinned_messages.then(|| {
            let label = if icon_buttons && pin_count > 0 {
                format!("📌 {pin_count}")
            } else if icon_buttons {
                "📌".to_string()
            } else if pin_count > 0 {
                format!("Pins {pin_count}")
            } else {
                "Pins".to_string()
            };
            self.header_button(
                &label,
                "Pinned messages",
                self.pins_open,
                cx,
                |this, _, cx| this.open_pins(cx),
            )
        });
        div()
            .h(px(52.))
            .flex_shrink_0()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .when(compact_header, |this| this.gap_2())
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
                            .min_w(px(0.))
                            .child(
                                div()
                                    .truncate()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
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
                            .when(topic_editable, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.text_color(theme.text))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.inputs.composer.update(cx, |input, cx| {
                                            input.set_content("/topic ", cx)
                                        });
                                        this.focus_composer(window, cx);
                                        cx.notify();
                                    }))
                            })
                            .child(gpui::StyledText::new(topic_label).into_any_element()),
                    ),
            )
            .when(channel.is_channel() && !compact_header, |this| {
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
            .when(
                (channel.is_channel() || channel.is_query()) && !self.narrow,
                |this| {
                    let muted = channel.chan.muted;
                    this.child(self.header_button(
                        if icon_buttons {
                            if muted { "🔇" } else { "🔊" }
                        } else if muted {
                            "Unmute"
                        } else {
                            "Mute"
                        },
                        if muted { "Unmute" } else { "Mute" },
                        muted,
                        cx,
                        move |this, _, cx| {
                            if let (Some(conn), Some(target)) = (&this.conn, this.state.active) {
                                conn.emit(
                                    "mute:change",
                                    Some(
                                        serde_json::json!({"target": target, "setMutedTo": !muted}),
                                    ),
                                );
                            }
                            cx.notify();
                        },
                    ))
                },
            )
            .when(
                (channel.is_channel() || channel.is_query()) && !self.narrow,
                |this| {
                    let is_query = channel.is_query();
                    let label = if icon_buttons {
                        "⏏"
                    } else if is_query {
                        "Close"
                    } else {
                        "Leave"
                    };
                    this.child(self.header_button(
                        label,
                        if is_query {
                            "Close query"
                        } else {
                            "Leave channel"
                        },
                        false,
                        cx,
                        move |this, _, cx| {
                            if let (Some(conn), Some(target)) = (&this.conn, this.state.active) {
                                conn.input(target, if is_query { "/close" } else { "/part" }, None);
                            }
                            cx.notify();
                        },
                    ))
                },
            )
            .child({
                let unread = self.unread_mentions();
                let label = if icon_buttons {
                    if unread > 0 {
                        format!("@ {unread}")
                    } else {
                        "@".to_string()
                    }
                } else if unread > 0 {
                    format!("Mentions {unread}")
                } else {
                    "Mentions".to_string()
                };
                self.header_button(
                    &label,
                    "Recent mentions",
                    self.mentions_open,
                    cx,
                    |this, _, cx| this.open_mentions(cx),
                )
            })
            .when_some(pins_button, |this, button| this.child(button))
            .child(self.header_button(
                if icon_buttons { "👥" } else { "Users" },
                "Toggle user list",
                users_shown,
                cx,
                {
                    let narrow = self.narrow;
                    move |this, _, cx| {
                        if narrow {
                            this.users_overlay = !this.users_overlay;
                            this.sidebar_overlay = false;
                        } else {
                            this.config.show_user_list = !this.config.show_user_list;
                            this.config.save();
                        }
                        cx.notify();
                    }
                },
            ))
            .child(self.header_button(
                if icon_buttons { "🔍" } else { "Search" },
                "Search messages",
                self.search_open,
                cx,
                |this, _, cx| this.open_search(cx),
            ))
            .into_any_element()
    }

    pub(super) fn sidebar_toggle(&self, cx: &mut Context<Self>) -> AnyElement {
        let open = if self.narrow {
            self.sidebar_overlay
        } else {
            self.config.show_sidebar
        };
        let label = if open { "←" } else { "☰" };
        let narrow = self.narrow;
        self.header_button(
            label,
            if open {
                "Collapse sidebar"
            } else {
                "Open sidebar"
            },
            false,
            cx,
            move |this, _, cx| {
                if narrow {
                    this.sidebar_overlay = !this.sidebar_overlay;
                    this.users_overlay = false;
                } else {
                    this.config.show_sidebar = !this.config.show_sidebar;
                    this.config.save();
                }
                cx.notify();
            },
        )
    }

    pub(super) fn header_button(
        &self,
        label: &str,
        tooltip: &str,
        active: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let tooltip = tooltip.to_string();
        let icon = match label {
            "🔇" => Some("volume-x"),
            "🔊" => Some("volume-2"),
            "⏏" => Some("log-out"),
            "👥" => Some("users"),
            "🔍" => Some("search"),
            "☰" => Some("menu"),
            "←" => Some("chevron-left"),
            value if value.starts_with('@') => Some("at-sign"),
            _ => None,
        };
        let content = if let Some(icon) = icon {
            let color = if active { theme.accent } else { theme.muted };
            let mut row = div().flex().items_center().gap_1();
            row = row.child(
                gpui::svg()
                    .path(format!("icons/{icon}.svg"))
                    .w(px(16.))
                    .h(px(16.))
                    .text_color(color),
            );
            let badge = label.strip_prefix('@').unwrap_or("").trim();
            if !badge.is_empty() {
                row = row.child(div().text_size(px(10.)).child(badge.to_string()));
            }
            row.into_any_element()
        } else {
            div().child(label.to_string()).into_any_element()
        };
        let action = std::rc::Rc::new(on_click);
        let click_action = action.clone();
        div()
            .id(ElementId::Name(format!("head-{label}").into()))
            .tab_index(0)
            .tab_stop(!self.last_overlay_open)
            .focus(|style| style.bg(theme.border).text_color(theme.text))
            .px_2()
            .py(px(4.))
            .rounded(px(4.))
            .text_size(px(14.))
            .cursor_pointer()
            .when(active, |this| {
                this.bg(theme.accent_dim).text_color(theme.accent)
            })
            .when(!active, |this| {
                this.text_color(theme.muted)
                    .hover(|style| style.bg(theme.hover))
            })
            .on_click(cx.listener(move |this, _, window, cx| click_action(this, window, cx)))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if activation_key(event) {
                        action(this, window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new(tooltip.clone(), theme)).into())
            .child(content)
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

        let composer_text = self.inputs.composer.read(cx).content().to_string();
        let emoji_token = composer_text
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or_default();
        let emoji_suggestions = if emoji_token.starts_with(':') && emoji_token.len() > 1 {
            crate::emoji::filter(emoji_token, 6)
        } else {
            Vec::new()
        };

        let mut footer = div()
            .flex_shrink_0()
            .min_h(px(COMPOSER_MIN_HEIGHT))
            .border_t_1()
            .border_color(theme.border)
            .p_2()
            .flex()
            .flex_col()
            .gap_2();

        if !emoji_suggestions.is_empty() && self.config.features.emoji_suggestions {
            let mut row = div().flex().flex_wrap().items_center().gap_1();
            row = row.child(
                div()
                    .text_size(px(10.))
                    .text_color(theme.muted)
                    .child("EMOJI"),
            );
            for emoji in emoji_suggestions {
                let glyph = emoji.glyph;
                let shortcode = emoji.shortcode;
                row = row.child(
                    div()
                        .id(ElementId::Name(
                            format!("composer-emoji-{shortcode}").into(),
                        ))
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .py(px(1.))
                        .rounded(px(4.))
                        .bg(theme.panel)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_emoji_completion(glyph, shortcode, cx)
                        }))
                        .child(div().text_size(px(16.)).child(glyph))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme.muted)
                                .child(shortcode),
                        ),
                );
            }
            footer = footer.child(row);
        }

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

        if let Some(upload) = &self.active_upload {
            let sent = upload.progress.sent();
            let total = upload.progress.total();
            let percent = sent
                .min(total)
                .saturating_mul(100)
                .checked_div(total)
                .unwrap_or(0) as u32;
            let progress = upload.progress.clone();
            footer = footer.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .bg(theme.panel)
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child(format!("Uploading {} ({percent}%)", upload.name))
                    .child(
                        div()
                            .id("cancel-upload")
                            .px_1()
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.error))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                progress.cancel();
                                cx.notify();
                            }))
                            .child("Cancel"),
                    ),
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
                            .child("×"),
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
            .tab_index(0)
            .tab_stop(!self.last_overlay_open)
            .focus(|style| style.bg(theme.border).text_color(theme.text))
            .h(px(34.))
            .px_2()
            .flex()
            .items_center()
            .rounded(px(6.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .text_size(px(16.))
            .text_color(theme.muted)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover).text_color(theme.text))
            .on_click(cx.listener(|this, _, _, cx| this.open_emoji_picker(cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if activation_key(event) {
                    this.open_emoji_picker(cx);
                    cx.stop_propagation();
                }
            }))
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new("Insert emoji", theme)).into())
            .child(if self.config.icon_buttons {
                "☺"
            } else {
                "Emoji"
            });
        let upload_button = upload_enabled.then(|| {
            div()
                .id("upload-file")
                .tab_index(0)
                .tab_stop(!self.last_overlay_open)
                .focus(|style| style.bg(theme.border).text_color(theme.text))
                .h(px(34.))
                .px_2()
                .flex()
                .items_center()
                .rounded(px(6.))
                .bg(theme.panel)
                .border_1()
                .border_color(theme.border)
                .text_size(px(16.))
                .text_color(theme.text)
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover).text_color(theme.text))
                .on_click(cx.listener(|this, _, _, cx| this.prompt_for_upload(cx)))
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                    if activation_key(event) {
                        this.prompt_for_upload(cx);
                        cx.stop_propagation();
                    }
                }))
                .tooltip(move |_, cx| cx.new(|_| Tooltip::new("Upload file", theme)).into())
                .child(if self.config.icon_buttons {
                    "📎"
                } else {
                    "Upload"
                })
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
}

fn channel_secondary_label(channel: &ChannelState) -> String {
    if channel.is_lobby() {
        return "Network".into();
    }
    if channel.is_query() {
        if channel.chan.name.eq_ignore_ascii_case("*status") {
            return "Status".into();
        }
        return "Private message".into();
    }
    match channel.chan.special.as_deref() {
        Some("list_bans") => "Ban list".into(),
        Some("list_invites") => "Invite list".into(),
        Some("list_channels") => "Channel list".into(),
        Some("list_ignored") => "Ignored users".into(),
        Some(_) => "Special channel".into(),
        None => "Channel".into(),
    }
}

fn compact_header(
    viewport_width: Pixels,
    narrow: bool,
    show_sidebar: bool,
    show_user_list: bool,
    is_channel: bool,
) -> bool {
    let reserved = if narrow {
        px(0.)
    } else {
        (if show_sidebar { px(238.) } else { px(0.) })
            + (if show_user_list && is_channel {
                px(210.)
            } else {
                px(0.)
            })
    };
    viewport_width - reserved < px(900.)
}

#[cfg(test)]
mod tests {
    use super::{channel_secondary_label, compact_header};
    use gpui::px;
    use lounge_core::proto::ChanType;
    use lounge_core::state::ChannelState;

    #[test]
    fn non_channel_headers_describe_the_view() {
        let mut channel = ChannelState::default();
        channel.chan.kind = ChanType::Query;
        channel.chan.name = "alice".into();
        assert_eq!(channel_secondary_label(&channel), "Private message");

        channel.chan.name = "*status".into();
        assert_eq!(channel_secondary_label(&channel), "Status");

        channel.chan.kind = ChanType::Lobby;
        assert_eq!(channel_secondary_label(&channel), "Network");

        channel.chan.kind = ChanType::Special;
        channel.chan.special = Some("list_ignored".into());
        assert_eq!(channel_secondary_label(&channel), "Ignored users");
    }

    #[test]
    fn compact_header_uses_chat_width() {
        assert!(compact_header(px(1347.), false, true, true, true));
        assert!(!compact_header(px(1348.), false, true, true, true));
    }

    #[test]
    fn compact_header_ignores_hidden_side_panels() {
        assert!(!compact_header(px(900.), false, false, false, true));
        assert!(compact_header(px(899.), false, false, false, true));
        assert!(compact_header(px(899.), true, true, true, true));
        assert!(!compact_header(px(900.), true, true, true, true));
    }
}
