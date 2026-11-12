use super::*;

impl LoungeApp {
    pub(super) fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.sidebar_overlay = false;
        self.users_overlay = false;
        self.nav_mode = false;
        self.settings_open = true;
        self.settings_refocus = true;
        let highlights = self
            .state
            .settings
            .get("highlights")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let exceptions = self
            .state
            .settings
            .get("highlightExceptions")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let away = self
            .state
            .settings
            .get("awayMessage")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        self.inputs
            .setting_highlights
            .update(cx, |input, cx| input.set_content(highlights, cx));
        self.inputs
            .setting_highlight_exceptions
            .update(cx, |input, cx| input.set_content(exceptions, cx));
        self.inputs
            .setting_away_message
            .update(cx, |input, cx| input.set_content(away, cx));
        cx.notify();
    }

    pub(super) fn open_network_modal(&mut self, cx: &mut Context<Self>) {
        self.network_modal_open = true;
        self.network_edit_uuid = None;
        self.network_edit_data = None;
        self.network_edit_loading = false;
        self.network_has_sts = false;
        self.network_tls = true;
        self.network_defaults_applied = false;
        self.focus_request = Some(InputRole::NetworkHost);
        for input in [
            &self.inputs.network_name,
            &self.inputs.network_host,
            &self.inputs.network_port,
            &self.inputs.network_nick,
            &self.inputs.network_username,
            &self.inputs.network_realname,
            &self.inputs.network_password,
        ] {
            input.update(cx, |input, cx| input.set_content("", cx));
        }
        if !self.network_defaults_applied {
            self.network_defaults_applied = true;
            let defaults = self
                .state
                .configuration
                .as_ref()
                .and_then(|config| config.defaults.clone());
            if let Some(defaults) = defaults {
                self.inputs.network_host.update(cx, |input, cx| {
                    input.set_content(
                        if defaults.host.is_empty() {
                            "irc.libera.chat".to_string()
                        } else {
                            defaults.host.clone()
                        },
                        cx,
                    );
                });
                self.inputs.network_port.update(cx, |input, cx| {
                    input.set_content(
                        if defaults.port == 0 {
                            "6697".to_string()
                        } else {
                            defaults.port.to_string()
                        },
                        cx,
                    );
                });
                self.inputs.network_nick.update(cx, |input, cx| {
                    input.set_content(defaults.nick.clone(), cx);
                });
                self.inputs.network_username.update(cx, |input, cx| {
                    input.set_content(defaults.username.clone(), cx);
                });
                self.inputs.network_realname.update(cx, |input, cx| {
                    input.set_content(defaults.realname.clone(), cx);
                });
                self.network_tls = defaults.tls;
            } else {
                self.inputs.network_host.update(cx, |input, cx| {
                    input.set_content("irc.libera.chat", cx);
                });
                self.inputs.network_port.update(cx, |input, cx| {
                    input.set_content("6697", cx);
                });
                self.inputs.network_nick.update(cx, |input, cx| {
                    input.set_content(self.config.user.clone(), cx);
                });
            }
        }
        cx.notify();
    }

    pub(super) fn open_search(&mut self, cx: &mut Context<Self>) {
        self.search_open = true;
        self.search_selection = 0;
        self.search_scroll = gpui::ScrollHandle::new();
        self.focus_request = Some(InputRole::Search);
        self.state.search_results = None;
        cx.notify();
    }

    pub(super) fn open_mentions(&mut self, cx: &mut Context<Self>) {
        self.mentions_open = true;
        self.mentions_selection = 0;
        self.mentions_scroll = gpui::ScrollHandle::new();
        self.focus_root_request = true;
        self.mark_mentions_seen();
        if let Some(conn) = &self.conn {
            conn.emit("mentions:get", None);
        }
        cx.notify();
    }

    pub(super) fn open_emoji_picker(&mut self, cx: &mut Context<Self>) {
        self.emoji_picker_open = true;
        self.emoji_selection = 0;
        self.emoji_scroll = gpui::ScrollHandle::new();
        self.focus_request = Some(InputRole::EmojiSearch);
        self.inputs
            .emoji_search
            .update(cx, |input, cx| input.set_content("", cx));
        cx.notify();
    }

    pub(super) fn open_font_picker(&mut self, target: FontTarget, cx: &mut Context<Self>) {
        self.font_picker = Some(target);
        self.focus_request = Some(InputRole::FontSearch);
        self.inputs
            .font_search
            .update(cx, |input, cx| input.set_content("", cx));
        cx.notify();
    }

    pub(super) fn choose_font(&mut self, target: FontTarget, font: &str, cx: &mut Context<Self>) {
        match target {
            FontTarget::Ui => self.config.font_family = font.to_string(),
            FontTarget::Mono => self.config.monospace_font = font.to_string(),
        }
        self.apply_theme(cx);
        self.config.save();
        self.font_picker = None;
        self.settings_open = true;
        self.last_overlay_open = false;
        self.pending_focus = false;
        cx.notify();
    }

    pub(super) fn insert_emoji_with_shortcode(
        &mut self,
        glyph: &str,
        shortcode: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.remember_emoji(shortcode);
        self.insert_emoji(glyph, cx);
    }

    pub(super) fn remember_emoji(&mut self, shortcode: &'static str) {
        if !self.config.features.recent_emoji {
            return;
        }
        self.recent_emoji.retain(|code| *code != shortcode);
        self.recent_emoji.insert(0, shortcode);
        self.recent_emoji.truncate(12);
    }

    pub(super) fn open_pins(&mut self, cx: &mut Context<Self>) {
        self.pins_open = true;
        cx.notify();
    }

    pub(super) fn is_message_pinned(&self, channel: i64, message: i64) -> bool {
        self.config
            .pinned_messages
            .iter()
            .any(|pin| pin.channel == channel && pin.message == message)
    }

    pub(super) fn toggle_pinned_message(
        &mut self,
        channel: i64,
        message: &Message,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self
            .config
            .pinned_messages
            .iter()
            .position(|pin| pin.channel == channel && pin.message == message.id)
        {
            self.config.pinned_messages.remove(index);
            self.toast("Message unpinned.");
        } else {
            self.config
                .pinned_messages
                .push(crate::config::PinnedMessage {
                    channel,
                    message: message.id,
                    nick: message
                        .from
                        .as_ref()
                        .map(|from| from.nick.clone())
                        .unwrap_or_default(),
                    text: format::clean(&message.text).chars().take(1000).collect(),
                    time: format::format_time(
                        message.time,
                        self.config.show_seconds,
                        self.config.use_12h_clock,
                    ),
                });
            self.toast("Message pinned.");
        }
        self.config.save();
        self.message_menu = None;
        cx.notify();
    }

    pub(super) fn unpin_message(&mut self, channel: i64, message: i64, cx: &mut Context<Self>) {
        self.config
            .pinned_messages
            .retain(|pin| !(pin.channel == channel && pin.message == message));
        self.config.save();
        self.toast("Message unpinned.");
        cx.notify();
    }

    pub(super) fn insert_emoji(&mut self, glyph: &str, cx: &mut Context<Self>) {
        let glyph = glyph.to_string();
        self.inputs
            .composer
            .update(cx, |input, cx| input.insert_text(&glyph, cx));
        self.emoji_picker_open = false;
        self.pending_focus = true;
        cx.notify();
    }

    pub(super) fn drag_preview(
        drag: &SortDrag,
        position: Point<Pixels>,
        theme: Theme,
        cx: &mut App,
    ) -> Entity<DragPreview> {
        cx.new(|_| DragPreview {
            label: drag.label.clone().into(),
            position,
            color: theme.accent.opacity(0.85),
            text_color: theme.on_accent,
        })
    }

    pub(super) fn drop_sort(
        &mut self,
        drag: &SortDrag,
        target: SortTarget,
        cx: &mut Context<Self>,
    ) {
        match (drag.kind, target) {
            (SortKind::Network, SortTarget::Network(target_uuid)) => {
                if drag.network_uuid == target_uuid {
                    return;
                }
                let mut order: Vec<String> = self
                    .state
                    .networks
                    .iter()
                    .map(|network| network.uuid.clone())
                    .collect();
                let source_index = order.iter().position(|uuid| *uuid == drag.network_uuid);
                if let Some(index) = source_index {
                    order.remove(index);
                }
                let target_index = order
                    .iter()
                    .position(|uuid| *uuid == target_uuid)
                    .unwrap_or(order.len());
                let insert_at = if source_index.unwrap_or(0) < target_index {
                    (target_index + 1).min(order.len())
                } else {
                    target_index
                };
                order.insert(insert_at, drag.network_uuid.clone());
                if let Some(conn) = &self.conn {
                    conn.emit("sort:networks", Some(serde_json::json!({"order": order})));
                }
            }
            (
                SortKind::Channel,
                SortTarget::Channel {
                    network_uuid,
                    channel_id,
                },
            ) => {
                if drag.network_uuid != network_uuid || drag.channel_id == Some(channel_id) {
                    return;
                }
                let Some(network) = self
                    .state
                    .networks
                    .iter()
                    .find(|network| network.uuid == network_uuid)
                else {
                    return;
                };
                let mut order: Vec<i64> = network
                    .channels
                    .iter()
                    .filter(|channel| {
                        matches!(
                            channel.chan.kind,
                            lounge_core::ChanType::Channel | lounge_core::ChanType::Query
                        )
                    })
                    .map(|channel| channel.chan.id)
                    .collect();
                let Some(drag_id) = drag.channel_id else {
                    return;
                };
                let source_index = order.iter().position(|id| *id == drag_id);
                if let Some(index) = source_index {
                    order.remove(index);
                }
                let target_index = order
                    .iter()
                    .position(|id| *id == channel_id)
                    .unwrap_or(order.len());
                let insert_at = if source_index.unwrap_or(0) < target_index {
                    (target_index + 1).min(order.len())
                } else {
                    target_index
                };
                order.insert(insert_at, drag_id);
                for channel in &network.channels {
                    if !matches!(
                        channel.chan.kind,
                        lounge_core::ChanType::Channel | lounge_core::ChanType::Query
                    ) {
                        order.push(channel.chan.id);
                    }
                }
                if let Some(conn) = &self.conn {
                    conn.emit(
                        "sort:channels",
                        Some(serde_json::json!({"network": network_uuid, "order": order})),
                    );
                }
            }
            _ => {}
        }
        cx.notify();
    }

    pub(super) fn open_network_menu(&mut self, uuid: &str, cx: &mut Context<Self>) {
        self.open_network_menu_at(uuid, None, cx);
    }

    pub(super) fn open_network_menu_at(
        &mut self,
        uuid: &str,
        position: Option<Point<Pixels>>,
        cx: &mut Context<Self>,
    ) {
        self.network_menu = Some(NetworkMenu {
            uuid: uuid.to_string(),
            position,
        });
        self.confirm_remove_network = None;
        cx.notify();
    }

    pub(super) fn network_lobby(&self, uuid: &str) -> Option<i64> {
        self.state
            .networks
            .iter()
            .find(|network| network.uuid == uuid)
            .and_then(|network| network.channels.first())
            .map(|channel| channel.chan.id)
    }

    pub(super) fn send_network_command(
        &mut self,
        uuid: &str,
        command: &str,
        cx: &mut Context<Self>,
    ) {
        if let (Some(conn), Some(lobby)) = (&self.conn, self.network_lobby(uuid)) {
            conn.input(lobby, command, None);
        }
        self.network_menu = None;
        self.confirm_remove_network = None;
        self.pending_focus = true;
        cx.notify();
    }

    pub(super) fn begin_edit_network(&mut self, uuid: &str, cx: &mut Context<Self>) {
        self.network_menu = None;
        self.network_edit_uuid = Some(uuid.to_string());
        self.network_edit_data = None;
        self.network_edit_loading = true;
        self.network_defaults_applied = true;
        self.network_modal_open = true;
        self.focus_request = Some(InputRole::NetworkHost);
        if let Some(conn) = &self.conn {
            conn.emit("network:get", Some(serde_json::json!(uuid)));
        }
        cx.notify();
    }

    pub(super) fn prefill_network_edit(
        &mut self,
        info: &lounge_core::NetworkInfo,
        cx: &mut Context<Self>,
    ) {
        if self.network_edit_uuid.as_deref() != Some(info.uuid.as_str()) {
            return;
        }
        let data = &info.data;
        let get_str = |key: &str| {
            data.get(key)
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string()
        };
        let get_bool = |key: &str| {
            data.get(key)
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        };
        self.network_edit_uuid = Some(info.uuid.clone());
        self.network_edit_data = Some(info.data.clone());
        self.network_edit_loading = false;
        self.network_defaults_applied = true;
        self.network_has_sts = get_bool("hasSTSPolicy");
        self.network_tls = get_bool("tls");
        self.inputs
            .network_name
            .update(cx, |input, cx| input.set_content(get_str("name"), cx));
        self.inputs
            .network_host
            .update(cx, |input, cx| input.set_content(get_str("host"), cx));
        self.inputs.network_port.update(cx, |input, cx| {
            let port = data
                .get("port")
                .and_then(|value| value.as_u64())
                .unwrap_or(6697);
            input.set_content(port.to_string(), cx)
        });
        self.inputs
            .network_nick
            .update(cx, |input, cx| input.set_content(get_str("nick"), cx));
        self.inputs
            .network_username
            .update(cx, |input, cx| input.set_content(get_str("username"), cx));
        self.inputs
            .network_realname
            .update(cx, |input, cx| input.set_content(get_str("realname"), cx));
        self.inputs
            .network_password
            .update(cx, |input, cx| input.set_content(get_str("password"), cx));
        self.network_modal_open = true;
        cx.notify();
    }

    pub(super) fn open_user_menu(
        &mut self,
        nick: &str,
        network_uuid: &str,
        chan_id: i64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.user_menu = Some(UserMenu {
            nick: nick.to_string(),
            network_uuid: network_uuid.to_string(),
            chan_id,
            position,
        });
        cx.notify();
    }

    pub(super) fn send_user_command(
        &mut self,
        menu: &UserMenu,
        command: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(conn) = &self.conn {
            conn.input(menu.chan_id, format!("/{command} {}", menu.nick), None);
        }
        self.user_menu = None;
        cx.notify();
    }

    pub(super) fn open_channel_menu(&mut self, menu: ChannelMenu, cx: &mut Context<Self>) {
        self.channel_menu = Some(menu);
        cx.notify();
    }

    pub(super) fn move_network(&mut self, uuid: &str, delta: i32, cx: &mut Context<Self>) {
        let mut order: Vec<String> = self
            .state
            .networks
            .iter()
            .map(|network| network.uuid.clone())
            .collect();
        let Some(position) = order.iter().position(|id| id == uuid) else {
            return;
        };
        let target = position as i32 + delta;
        if target < 0 || target as usize >= order.len() {
            return;
        }
        order.swap(position, target as usize);
        if let Some(conn) = &self.conn {
            conn.emit("sort:networks", Some(serde_json::json!({"order": order})));
        }
        cx.notify();
    }

    pub(super) fn move_channel(
        &mut self,
        network_uuid: &str,
        chan_id: i64,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let Some(network) = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == network_uuid)
        else {
            return;
        };
        let mut order: Vec<i64> = network
            .channels
            .iter()
            .filter(|channel| {
                matches!(
                    channel.chan.kind,
                    lounge_core::ChanType::Channel | lounge_core::ChanType::Query
                )
            })
            .map(|channel| channel.chan.id)
            .collect();
        let Some(position) = order.iter().position(|id| *id == chan_id) else {
            return;
        };
        let target = position as i32 + delta;
        if target < 0 || target as usize >= order.len() {
            return;
        }
        order.swap(position, target as usize);
        for channel in &network.channels {
            if !matches!(
                channel.chan.kind,
                lounge_core::ChanType::Channel | lounge_core::ChanType::Query
            ) {
                order.push(channel.chan.id);
            }
        }
        if let Some(conn) = &self.conn {
            conn.emit(
                "sort:channels",
                Some(serde_json::json!({"network": network_uuid, "order": order})),
            );
        }
        cx.notify();
    }

    pub(super) fn close_overlays(&mut self, cx: &mut Context<Self>) {
        if self.theme_editor.is_some() {
            self.cancel_theme_editor(cx);
        }
        let return_hotkey_settings =
            self.palette == Some(PaletteMode::Hotkey) && self.hotkey_settings_return;
        self.last_overlay_open = false;
        self.sidebar_overlay = false;
        self.users_overlay = false;
        if self.font_picker.is_some() || self.security_open {
            self.font_picker = None;
            self.security_open = false;
            self.settings_open = true;
            self.pending_focus = false;
            cx.notify();
            return;
        }
        #[cfg(feature = "media")]
        self.clear_media();
        self.settings_open = false;
        self.network_modal_open = false;
        self.network_edit_uuid = None;
        self.network_edit_data = None;
        self.network_edit_loading = false;
        self.search_open = false;
        self.mentions_open = false;
        self.pins_open = false;
        self.emoji_picker_open = false;
        self.font_picker = None;
        self.security_open = false;
        self.network_menu = None;
        self.user_menu = None;
        self.friend_menu = None;
        self.channel_menu = None;
        self.message_menu = None;
        self.confirm_remove_network = None;
        self.lightbox = None;
        self.palette = None;
        self.message_context = None;
        self.pending_context = None;
        self.shortcuts_open = false;
        self.hotkey_settings_return = false;
        if return_hotkey_settings {
            self.settings_open = true;
            self.settings_tab = SettingsTab::General;
            self.focus_root_request = true;
        } else {
            self.pending_focus = true;
        }
        cx.notify();
    }

    pub(super) fn render_network_menu(
        &mut self,
        menu: NetworkMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let uuid = menu.uuid;
        let position = menu.position;
        let theme = self.theme;
        let network_name = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == uuid)
            .map(|network| network.name.clone())
            .unwrap_or_else(|| uuid.to_string());
        let locked = self
            .state
            .configuration
            .as_ref()
            .map(|config| config.lock_network)
            .unwrap_or(false);
        let uuid_edit = uuid.to_string();
        let uuid_connect = uuid.to_string();
        let uuid_disconnect = uuid.to_string();
        let uuid_ignored = uuid.to_string();
        let uuid_remove = uuid.to_string();
        let uuid_cancel = uuid.to_string();
        let uuid_favourite = uuid.to_string();
        let favourite_label = if self.is_favourite_network(&uuid) {
            "Remove from favourites"
        } else {
            "Add to favourites"
        };

        let mut card = self.menu_card(&network_name, cx);
        if !locked {
            card = card.child(self.menu_button("Edit network", cx, move |this, _, cx| {
                this.begin_edit_network(&uuid_edit, cx)
            }));
        }
        card = card
            .child(self.menu_button("Reconnect", cx, move |this, _, cx| {
                this.send_network_command(&uuid_connect, "/connect", cx)
            }))
            .child(self.menu_button(favourite_label, cx, move |this, _, cx| {
                this.toggle_favourite_network(&uuid_favourite, cx);
                this.network_menu = None;
            }))
            .child(self.menu_button("Move network up", cx, {
                let uuid = uuid.to_string();
                move |this, _, cx| this.move_network(&uuid, -1, cx)
            }))
            .child(self.menu_button("Move network down", cx, {
                let uuid = uuid.to_string();
                move |this, _, cx| this.move_network(&uuid, 1, cx)
            }))
            .child(self.menu_button("Disconnect", cx, move |this, _, cx| {
                this.send_network_command(&uuid_disconnect, "/disconnect", cx)
            }))
            .child(
                self.menu_button("List ignored users", cx, move |this, _, cx| {
                    this.send_network_command(&uuid_ignored, "/ignorelist", cx)
                }),
            )
            .child(
                div()
                    .id("network-remove")
                    .px_2()
                    .py(px(7.))
                    .rounded(px(5.))
                    .cursor_pointer()
                    .text_color(theme.error)
                    .hover(|style| style.bg(theme.hover))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.network_menu = None;
                        this.confirm_remove_network = Some(uuid_remove.clone());
                        cx.notify();
                    }))
                    .child("Remove network"),
            )
            .child(self.menu_button("Cancel", cx, move |this, _, cx| {
                this.network_menu = None;
                let _ = uuid_cancel;
                cx.notify();
            }));
        if let Some(position) = position {
            let (x, y) = menu_origin(window, position, 420., 500.);
            div()
                .absolute()
                .left(x)
                .top(y)
                .child(card.into_any_element())
                .into_any_element()
        } else {
            card.into_any_element()
        }
    }

    pub(super) fn render_remove_network(
        &mut self,
        uuid: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let network_name = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == uuid)
            .map(|network| network.name.clone())
            .unwrap_or_else(|| uuid.to_string());
        let uuid_remove = uuid.to_string();
        self.menu_card("Remove network", cx)
            .child(div().text_color(theme.muted).child(format!(
                "Remove {network_name}? This quits the IRC connection and closes its channels."
            )))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(self.menu_button("Cancel", cx, |this, _, cx| {
                        this.confirm_remove_network = None;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .id("confirm-remove-network")
                            .px_3()
                            .py(px(6.))
                            .rounded(px(5.))
                            .bg(theme.error)
                            .text_color(theme.bg)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.send_network_command(&uuid_remove, "/quit", cx)
                            }))
                            .child("Remove"),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_message_menu(
        &mut self,
        menu: MessageMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let position = menu.position;
        let message = menu.message.clone();
        let text = format::clean(&message.text);
        let nick = message
            .from
            .as_ref()
            .map(|from| from.nick.clone())
            .unwrap_or_default();
        let mut card = self.menu_card("Message", cx);
        card = card.child(self.menu_button("Copy message", cx, {
            let text = text.clone();
            move |this, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                this.message_menu = None;
                this.toast("Message copied.");
                cx.notify();
            }
        }));
        if !nick.is_empty() {
            card = card.child(self.menu_button("Copy nickname", cx, {
                let nick = nick.clone();
                move |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(nick.clone()));
                    this.message_menu = None;
                    this.toast("Nickname copied.");
                    cx.notify();
                }
            }));
        }
        if message.id > 0 {
            let link = format!(
                "{}/#/chan-{}?focused={}",
                self.config.server.trim_end_matches('/'),
                menu.channel_id,
                message.id
            );
            let copy_link = link.clone();
            card = card.child(self.menu_button("Copy link", cx, move |this, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copy_link.clone()));
                this.message_menu = None;
                this.toast("Message link copied.");
                cx.notify();
            }));
            card = card.child(
                self.menu_button("Open link in browser", cx, move |_, _, _| {
                    let _ = open::that(link.clone());
                }),
            );
        }
        if self.config.features.pinned_messages && message.id > 0 {
            let channel_id = menu.channel_id;
            let pinned = self.is_message_pinned(channel_id, message.id);
            let pin_message = message.clone();
            card = card.child(self.menu_button(
                if pinned {
                    "Unpin message"
                } else {
                    "Pin message"
                },
                cx,
                move |this, _, cx| this.toggle_pinned_message(channel_id, &pin_message, cx),
            ));
        }
        if message.msgid.is_some() {
            let message = message.clone();
            card = card.child(self.menu_button("Reply", cx, move |this, _, cx| {
                this.set_reply(&message, cx);
                this.message_menu = None;
            }));
        }
        if !nick.is_empty() {
            let channel_id = menu.channel_id;
            let ignore_nick = nick.clone();
            card = card.child(self.menu_button("Ignore user", cx, move |this, _, cx| {
                if let Some(conn) = &this.conn {
                    conn.input(channel_id, format!("/ignore {ignore_nick}"), None);
                }
                this.message_menu = None;
                this.toast(format!("Ignoring {ignore_nick}."));
                cx.notify();
            }));
        }
        let card = card.into_any_element();
        let (x, y) = menu_origin(window, position, 420., 340.);
        div()
            .absolute()
            .left(x)
            .top(y)
            .child(card)
            .into_any_element()
    }

    pub(super) fn render_user_menu(
        &mut self,
        menu: UserMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let position = menu.position;
        let nick = menu.nick.clone();
        let mention_nick = menu.nick.clone();
        let commands = [
            ("Open query", "query"),
            ("Whois", "whois"),
            ("Give op", "op"),
            ("Remove op", "deop"),
            ("Give voice", "voice"),
            ("Remove voice", "devoice"),
            ("Kick", "kick"),
            ("Ban", "ban"),
            ("Ignore user", "ignore"),
        ];
        let mut card = self.menu_card(&nick, cx);
        card = card.child(self.menu_button("Mention", cx, move |this, _, cx| {
            this.insert_nick(&mention_nick, cx);
            this.user_menu = None;
            cx.notify();
        }));
        let friend_network = menu.network_uuid.clone();
        let friend_nick = nick.clone();
        let friend_label = if self.is_friend(&friend_network, &friend_nick) {
            "Remove friend"
        } else {
            "Add friend"
        };
        card = card.child(self.menu_button(friend_label, cx, move |this, _, cx| {
            this.toggle_friend(&friend_network, &friend_nick, cx);
            this.user_menu = None;
        }));
        for (label, command) in commands {
            let command = command.to_string();
            let menu = menu.clone();
            card = card.child(self.menu_button(label, cx, move |this, _, cx| {
                this.send_user_command(&menu, &command, cx)
            }));
        }
        let card = card
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Commands act on the active channel."),
            )
            .into_any_element();
        let (x, y) = menu_origin(window, position, 420., 500.);
        div()
            .absolute()
            .left(x)
            .top(y)
            .child(card)
            .into_any_element()
    }

    pub(super) fn render_channel_menu(
        &mut self,
        menu: ChannelMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let position = menu.position;
        let chan_id = menu.chan_id;
        let network_up = menu.network_uuid.clone();
        let network_down = menu.network_uuid.clone();
        let muted = menu.muted;
        let is_query = menu.is_query;
        let network_favourite = menu.network_uuid.clone();
        let name_favourite = menu.name.clone();
        let network_friend = menu.network_uuid.clone();
        let name_friend = menu.name.clone();
        let friend_label = if self.is_friend(&network_friend, &name_friend) {
            "Remove friend"
        } else {
            "Add friend"
        };
        let favourite_label = if self.is_favourite_channel(&network_favourite, &name_favourite) {
            "Remove from favourites"
        } else {
            "Add to favourites"
        };
        let mut card = self.menu_card(&menu.name, cx);
        card = card
            .child(self.menu_button("Move up", cx, move |this, _, cx| {
                this.move_channel(&network_up, chan_id, -1, cx);
                this.channel_menu = None;
            }))
            .child(self.menu_button("Move down", cx, move |this, _, cx| {
                this.move_channel(&network_down, chan_id, 1, cx);
                this.channel_menu = None;
            }))
            .child(self.menu_button(
                if muted { "Unmute" } else { "Mute" },
                cx,
                move |this, _, cx| {
                    if let Some(conn) = &this.conn {
                        conn.emit(
                            "mute:change",
                            Some(serde_json::json!({"target": chan_id, "setMutedTo": !muted})),
                        );
                    }
                    this.channel_menu = None;
                    cx.notify();
                },
            ))
            .child(self.menu_button("Clear history", cx, move |this, _, cx| {
                if let Some(conn) = &this.conn {
                    conn.emit(
                        "history:clear",
                        Some(serde_json::json!({"target": chan_id})),
                    );
                }
                this.channel_menu = None;
                cx.notify();
            }));
        if is_query {
            card = card.child(self.menu_button(friend_label, cx, move |this, _, cx| {
                this.toggle_friend(&network_friend, &name_friend, cx);
                this.channel_menu = None;
            }));
        } else {
            card = card.child(self.menu_button(favourite_label, cx, move |this, _, cx| {
                this.toggle_favourite_channel(&network_favourite, &name_favourite, cx);
                this.channel_menu = None;
            }));
            card = card.child(self.menu_button("Edit topic", cx, move |this, window, cx| {
                this.inputs
                    .composer
                    .update(cx, |input, cx| input.set_content("/topic ", cx));
                this.focus_composer(window, cx);
                this.channel_menu = None;
                cx.notify();
            }));
        }
        card = card.child(self.menu_button(
            if is_query {
                "Close query"
            } else {
                "Leave channel"
            },
            cx,
            move |this, _, cx| {
                if let Some(conn) = &this.conn {
                    conn.input(chan_id, if is_query { "/close" } else { "/part" }, None);
                }
                this.channel_menu = None;
                cx.notify();
            },
        ));
        let card = card
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Moving uses the server's channel ordering."),
            )
            .into_any_element();
        let (x, y) = menu_origin(window, position, 420., 480.);
        div()
            .absolute()
            .left(x)
            .top(y)
            .child(card)
            .into_any_element()
    }
}

pub(super) fn menu_origin(
    window: &Window,
    position: Point<Pixels>,
    width: f32,
    height: f32,
) -> (Pixels, Pixels) {
    let viewport = window.viewport_size();
    let x = position.x.min((viewport.width - px(width)).max(px(8.)));
    let y = position.y.min((viewport.height - px(height)).max(px(8.)));
    (x, y)
}
