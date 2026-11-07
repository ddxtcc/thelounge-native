use super::*;

impl LoungeApp {
    pub(super) fn prefill_login(&mut self, cx: &mut Context<Self>) {
        let server = self.config.server.clone();
        let user = self.config.user.clone();
        self.inputs.login_server.update(cx, |input, cx| {
            input.set_content(server, cx);
        });
        self.inputs.login_user.update(cx, |input, cx| {
            input.set_content(user, cx);
        });
    }

    pub(super) fn input_colors(&self) -> InputColors {
        self.theme.input_colors()
    }

    pub(super) fn apply_theme(&mut self, cx: &mut Context<Self>) {
        self.theme = Theme::from_config(&self.config.theme, self.config.custom_theme.as_ref());
        let colors = self.input_colors();
        let (size, line) = (
            self.config.font_size,
            (self.config.font_size * self.config.line_height).max(20.0),
        );
        let all = [
            self.inputs.composer.clone(),
            self.inputs.login_server.clone(),
            self.inputs.login_user.clone(),
            self.inputs.login_password.clone(),
            self.inputs.network_name.clone(),
            self.inputs.network_host.clone(),
            self.inputs.network_port.clone(),
            self.inputs.network_nick.clone(),
            self.inputs.network_username.clone(),
            self.inputs.network_realname.clone(),
            self.inputs.network_password.clone(),
            self.inputs.search.clone(),
            self.inputs.setting_highlights.clone(),
            self.inputs.setting_highlight_exceptions.clone(),
            self.inputs.setting_away_message.clone(),
            self.inputs.emoji_search.clone(),
            self.inputs.font_search.clone(),
            self.inputs.password_old.clone(),
            self.inputs.password_new.clone(),
            self.inputs.password_verify.clone(),
        ];
        for input in all {
            input.update(cx, |input, cx| {
                input.set_colors(colors.clone());
                input.set_text_size(size, line);
                cx.notify();
            });
        }
        cx.notify();
    }

    pub(super) fn on_input_event(
        &mut self,
        role: InputRole,
        input: Entity<TextInput>,
        cx: &mut Context<Self>,
    ) {
        let (action, revision, content) = input.update(cx, |input, _| {
            (
                input.take_action(),
                input.revision,
                input.content().to_string(),
            )
        });
        let last = self.input_revisions.get(&role).copied();
        if last != Some(revision) {
            self.input_revisions.insert(role, revision);
            self.on_input_changed(role, &content, cx);
        }
        if let Some(action) = action {
            self.on_input_action(role, action, cx);
        }
    }

    pub(super) fn on_input_changed(
        &mut self,
        role: InputRole,
        content: &str,
        cx: &mut Context<Self>,
    ) {
        if matches!(
            role,
            InputRole::Search | InputRole::EmojiSearch | InputRole::FontSearch | InputRole::Palette
        ) {
            if role == InputRole::Palette && self.palette.is_some() {
                self.palette_selection = 0;
                self.palette_scroll.scroll_to_item(0);
            }
            cx.notify();
        }
        if role == InputRole::Composer {
            if let Some(active) = self.state.active {
                self.drafts.insert(active, content.to_string());
                let is_query_or_channel = self
                    .state
                    .find_channel(active)
                    .map(|(_, c)| {
                        matches!(
                            c.chan.kind,
                            lounge_core::ChanType::Channel | lounge_core::ChanType::Query
                        )
                    })
                    .unwrap_or(false);
                if is_query_or_channel && !content.trim().is_empty() {
                    self.send_typing(true, cx);
                }
            }
            cx.notify();
        }
    }

    pub(super) fn on_input_action(
        &mut self,
        role: InputRole,
        action: InputAction,
        cx: &mut Context<Self>,
    ) {
        if action == InputAction::Escape
            && (self.last_overlay_open || self.sidebar_overlay || self.users_overlay)
        {
            self.close_overlays(cx);
            return;
        }
        match role {
            InputRole::Composer => match action {
                InputAction::Submit => self.send_composer(cx),
                InputAction::Tab => self.complete(cx),
                InputAction::PrevHistory => self.history_prev(cx),
                InputAction::NextHistory => self.history_next(cx),
                InputAction::Escape => {
                    if self.reply_to.take().is_some() {
                        cx.notify();
                    } else {
                        self.inputs.composer.update(cx, |input, cx| input.clear(cx));
                    }
                }
            },
            InputRole::LoginServer | InputRole::LoginUser | InputRole::LoginPassword => {
                if action == InputAction::Submit {
                    self.attempt_login(cx);
                }
            }
            InputRole::NetworkName
            | InputRole::NetworkHost
            | InputRole::NetworkPort
            | InputRole::NetworkNick
            | InputRole::NetworkUsername
            | InputRole::NetworkRealname
            | InputRole::NetworkPassword => {
                if action == InputAction::Submit {
                    self.submit_network(cx);
                }
            }
            InputRole::Search => {
                if action == InputAction::Submit {
                    self.submit_search(cx);
                }
            }
            InputRole::SettingHighlights
            | InputRole::SettingHighlightExceptions
            | InputRole::SettingAwayMessage => {
                if action == InputAction::Submit {
                    self.save_server_setting(role, cx);
                }
            }
            InputRole::EmojiSearch | InputRole::FontSearch => {}
            InputRole::Palette => match action {
                InputAction::Submit => self.run_palette_selection(cx),
                InputAction::PrevHistory => self.move_palette(-1, cx),
                InputAction::NextHistory => self.move_palette(1, cx),
                _ => {}
            },
            InputRole::PasswordOld | InputRole::PasswordNew | InputRole::PasswordVerify => {
                if action == InputAction::Submit {
                    self.change_password(cx);
                }
            }
        }
    }

    pub(super) fn change_password(&mut self, cx: &mut Context<Self>) {
        let old_password = self.inputs.password_old.read(cx).content().to_string();
        let new_password = self.inputs.password_new.read(cx).content().to_string();
        let verify_password = self.inputs.password_verify.read(cx).content().to_string();
        if new_password.is_empty() || new_password != verify_password {
            self.toast("New passwords do not match.");
            return;
        }
        if let Some(conn) = &self.conn {
            conn.emit(
                "change-password",
                Some(serde_json::json!({
                    "old_password": old_password,
                    "new_password": new_password,
                    "verify_password": verify_password,
                })),
            );
        }
    }

    pub(super) fn save_server_setting(&mut self, role: InputRole, cx: &mut Context<Self>) {
        let (name, input) = match role {
            InputRole::SettingHighlights => ("highlights", &self.inputs.setting_highlights),
            InputRole::SettingHighlightExceptions => (
                "highlightExceptions",
                &self.inputs.setting_highlight_exceptions,
            ),
            InputRole::SettingAwayMessage => ("awayMessage", &self.inputs.setting_away_message),
            _ => return,
        };
        let value = input.read(cx).content().to_string();
        if let Some(conn) = &self.conn {
            conn.emit(
                "setting:set",
                Some(serde_json::json!({"name": name, "value": value})),
            );
            self.state
                .settings
                .insert(name.to_string(), serde_json::Value::String(value));
            self.toast("Server setting saved.");
        }
    }

    pub(super) fn send_composer(&mut self, cx: &mut Context<Self>) {
        let text = self.inputs.composer.read(cx).content().to_string();
        let text = text.trim_end().to_string();
        if text.is_empty() {
            return;
        }
        let Some(target) = self.state.active else {
            return;
        };
        let reply_to = self.reply_to.as_ref().map(|reply| reply.msgid.clone());
        if let Some(conn) = &self.conn {
            conn.input(target, text.clone(), reply_to);
            conn.emit(
                "typing",
                Some(serde_json::json!({"target": target, "status": "done"})),
            );
        }
        self.typing_active = false;
        let history = self.histories.entry(target).or_default();
        if history.last() != Some(&text) {
            history.push(text);
            if history.len() > 200 {
                history.remove(0);
            }
        }
        self.history_index = 0;
        self.history_draft.clear();
        self.drafts.remove(&target);
        self.reply_to = None;
        self.inputs.composer.update(cx, |input, cx| input.clear(cx));
        self.schedule_scroll_bottom(cx);
        cx.notify();
    }

    pub(super) fn send_typing(&mut self, active: bool, cx: &mut Context<Self>) {
        let Some(target) = self.state.active else {
            return;
        };
        let Some(conn) = &self.conn else {
            return;
        };
        let now = Instant::now();
        if active {
            if self
                .last_typing_sent
                .map(|last| now.duration_since(last) < Duration::from_secs(3))
                .unwrap_or(false)
            {
                return;
            }
            self.last_typing_sent = Some(now);
            self.typing_active = true;
            conn.emit(
                "typing",
                Some(serde_json::json!({"target": target, "status": "active"})),
            );
            self.typing_generation = self.typing_generation.wrapping_add(1);
            let generation = self.typing_generation;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_secs(4)).await;
                let _ = this.update(cx, |this, cx| {
                    if this.typing_generation == generation && this.typing_active {
                        this.typing_active = false;
                        if let (Some(conn), Some(target)) = (&this.conn, this.state.active) {
                            conn.emit(
                                "typing",
                                Some(serde_json::json!({"target": target, "status": "done"})),
                            );
                        }
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }

    fn channel_ids(&self) -> Vec<i64> {
        self.state
            .networks
            .iter()
            .flat_map(|network| network.channels.iter().map(|channel| channel.chan.id))
            .collect()
    }

    pub(super) fn cycle_channel(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ids = self.channel_ids();
        if ids.is_empty() {
            return;
        }
        let current = self
            .state
            .active
            .and_then(|active| ids.iter().position(|id| *id == active))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(ids.len() as isize) as usize;
        self.switch_channel(ids[next], window, cx);
    }

    pub(super) fn switch_channel_by_index(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.channel_ids().get(index).copied() {
            self.switch_channel(id, window, cx);
        }
    }

    pub(super) fn switch_channel(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.switch_channel_quiet(id, window, cx);
        self.focus_composer(window, cx);
    }

    pub(super) fn switch_channel_quiet(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar_overlay = false;
        self.users_overlay = false;
        self.palette = None;
        self.text_selection = None;
        self.hovered_text = None;
        self.selecting_text = false;
        if self.state.active == Some(id) {
            return;
        }
        self.image_cache
            .update(cx, |cache, cx| cache.clear(window, cx));
        self.last_scroll_max = gpui::Size {
            width: px(0.),
            height: px(0.),
        };
        self.save_draft(cx);
        self.scroll_after_more = false;
        self.state.set_active(id);
        self.config.last_channel = Some(id);
        self.config.save();
        if let Some(conn) = &self.conn {
            conn.open(id);
        }
        self.open_active_channel(cx, true);
        self.restore_draft(cx);
        self.schedule_scroll_bottom(cx);
        self.reply_to = None;
        self.history_index = 0;
        self.pending_focus = true;
        cx.notify();
    }

    pub(super) fn save_draft(&mut self, cx: &mut Context<Self>) {
        if let Some(active) = self.state.active {
            let text = self.inputs.composer.read(cx).content().to_string();
            if text.trim().is_empty() {
                self.drafts.remove(&active);
            } else {
                self.drafts.insert(active, text);
            }
        }
    }

    pub(super) fn restore_draft(&mut self, cx: &mut Context<Self>) {
        let text = self
            .state
            .active
            .and_then(|id| self.drafts.get(&id).cloned())
            .unwrap_or_default();
        self.history_index = 0;
        self.history_draft.clear();
        self.inputs.composer.update(cx, |input, cx| {
            input.set_content(text, cx);
        });
    }

    pub(super) fn focus_composer(&self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.inputs.composer.read(cx).focus_handle(cx);
        window.focus(&handle);
    }

    pub(super) fn history_prev(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.state.active else {
            return;
        };
        let history = self.histories.get(&target).cloned().unwrap_or_default();
        if history.is_empty() {
            return;
        }
        if self.history_index == 0 {
            self.history_draft = self.inputs.composer.read(cx).content().to_string();
        }
        self.history_index = (self.history_index + 1).min(history.len());
        let text = history[history.len() - self.history_index].clone();
        self.inputs
            .composer
            .update(cx, |input, cx| input.set_content(text, cx));
    }

    pub(super) fn history_next(&mut self, cx: &mut Context<Self>) {
        if self.history_index == 0 {
            return;
        }
        self.history_index -= 1;
        let text = if self.history_index == 0 {
            self.history_draft.clone()
        } else {
            let Some(target) = self.state.active else {
                return;
            };
            let history = self.histories.get(&target).cloned().unwrap_or_default();
            history[history.len().saturating_sub(self.history_index)].clone()
        };
        self.inputs
            .composer
            .update(cx, |input, cx| input.set_content(text, cx));
    }

    pub(super) fn complete(&mut self, cx: &mut Context<Self>) {
        let content = self.inputs.composer.read(cx).content().to_string();
        let trimmed_end = content.trim_end_matches(' ').len();
        let token_start = content[..trimmed_end]
            .rfind(char::is_whitespace)
            .map(|index| index + 1)
            .unwrap_or(0);
        let token = &content[token_start..trimmed_end];

        if token.starts_with(':')
            && token.len() > 1
            && let Some(emoji) = crate::emoji::find(token)
        {
            let new_content = format!("{}{} ", &content[..token_start], emoji.glyph);
            self.inputs
                .composer
                .update(cx, |input, cx| input.set_content(new_content, cx));
            return;
        }

        let mut candidates: Vec<String> = Vec::new();
        if token.starts_with('/') && !content[..token_start].contains(' ') {
            candidates = self
                .state
                .commands
                .iter()
                .filter(|command| command.starts_with(token))
                .cloned()
                .collect();
            if candidates.is_empty() {
                candidates = self.state.commands.clone();
            }
        } else if !token.is_empty() {
            let nicks = self.state.nicks_for_active();
            candidates = nicks
                .into_iter()
                .filter(|nick| nick.to_lowercase().starts_with(&token.to_lowercase()))
                .collect();
        }

        if candidates.is_empty() {
            return;
        }
        candidates.sort_by_key(|candidate| candidate.to_lowercase());
        let completion = &candidates[0];
        let suffix = if token_start == 0 && !completion.starts_with('/') {
            ": "
        } else {
            " "
        };
        let new_content = format!("{}{}{}", &content[..token_start], completion, suffix);
        self.inputs
            .composer
            .update(cx, |input, cx| input.set_content(new_content, cx));
    }

    pub(super) fn insert_nick(&mut self, nick: &str, cx: &mut Context<Self>) {
        let content = self.inputs.composer.read(cx).content().to_string();
        let prefix = if content.is_empty() || content.ends_with(' ') {
            String::new()
        } else {
            " ".into()
        };
        let new_content = format!("{content}{prefix}{nick}: ");
        self.inputs
            .composer
            .update(cx, |input, cx| input.set_content(new_content, cx));
    }

    pub(super) fn set_reply(&mut self, message: &Message, cx: &mut Context<Self>) {
        let Some(msgid) = message.msgid.clone() else {
            return;
        };
        let nick = message
            .from
            .as_ref()
            .map(|from| from.nick.clone())
            .unwrap_or_default();
        let text = format::clean(&message.text);
        self.reply_to = Some(ReplyTarget {
            msgid,
            nick,
            text: text.chars().take(160).collect(),
        });
        self.pending_focus = true;
        cx.notify();
    }

    pub(super) fn absolute_media_url(&self, url: &str) -> String {
        if url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("data:")
            || url.starts_with("file:")
        {
            return url.to_string();
        }
        url::Url::parse(&format!("{}/", self.config.server.trim_end_matches('/')))
            .ok()
            .and_then(|base| base.join(url).ok())
            .map(|url| url.to_string())
            .unwrap_or_else(|| url.to_string())
    }

    pub(super) fn schedule_scroll_bottom(&mut self, _cx: &mut Context<Self>) {
        self.messages_scroll.scroll_to_bottom();
    }

    pub(super) fn copy_text_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let copied = {
            let Some(active) = self.state.active else {
                return false;
            };
            let Some((network, channel)) = self.state.find_channel(active) else {
                return false;
            };
            let Some(selection) = &self.text_selection else {
                return false;
            };
            let nick_column = self.config.nick_column && !self.narrow;
            let mut parts = Vec::new();
            for message in &channel.chan.messages {
                let (text, _, _, _) = super::message_text::build_message_text(
                    message,
                    network,
                    channel,
                    &self.config,
                    nick_column,
                    &self.theme,
                    self.theme.dark,
                );
                if let Some(range) = selection.range_for(channel, message.id, text.len())
                    && !range.is_empty()
                {
                    parts.push(text[range].to_string());
                }
            }
            parts.join("\n")
        };
        if copied.is_empty() {
            return false;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(copied));
        self.toast("Copied selection.");
        cx.notify();
        true
    }

    pub(super) fn scroll_messages_page(&mut self, direction: f32, cx: &mut Context<Self>) {
        let offset = self.messages_scroll.offset();
        let max = self.messages_scroll.max_offset();
        let height = self.messages_scroll.bounds().size.height;
        let mut y = offset.y + height * direction;
        if y > px(0.) {
            y = px(0.);
        }
        if y < -max.height {
            y = -max.height;
        }
        self.messages_scroll.set_offset(Point { x: offset.x, y });
        cx.notify();
    }

    pub(super) fn at_bottom(&self) -> bool {
        let offset = self.messages_scroll.offset();
        let max = self.messages_scroll.max_offset();
        max.height <= px(1.) || offset.y <= -max.height + px(48.)
    }

    pub(super) fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            until: Instant::now() + Duration::from_secs(6),
        });
    }

    pub(super) fn submit_network(&mut self, cx: &mut Context<Self>) {
        let name = self
            .inputs
            .network_name
            .read(cx)
            .content()
            .trim()
            .to_string();
        let host = self
            .inputs
            .network_host
            .read(cx)
            .content()
            .trim()
            .to_string();
        let port_text = self
            .inputs
            .network_port
            .read(cx)
            .content()
            .trim()
            .to_string();
        let nick = self
            .inputs
            .network_nick
            .read(cx)
            .content()
            .trim()
            .to_string();
        let username = self
            .inputs
            .network_username
            .read(cx)
            .content()
            .trim()
            .to_string();
        let realname = self
            .inputs
            .network_realname
            .read(cx)
            .content()
            .trim()
            .to_string();
        let password = self.inputs.network_password.read(cx).content().to_string();

        let Some(conn) = &self.conn else {
            return;
        };
        if nick.is_empty() {
            self.toast("Nickname is required.");
            return;
        }
        let mut data = serde_json::json!({
            "name": name,
            "nick": nick,
            "username": username,
            "realname": realname,
            "password": password,
            "tls": self.network_tls,
            "rejectUnauthorized": true,
        });
        if !host.is_empty() {
            data["host"] = serde_json::json!(host);
            if let Ok(port) = port_text.parse::<u16>() {
                data["port"] = serde_json::json!(port);
            }
        }
        if let Some(uuid) = self.network_edit_uuid.clone() {
            data["uuid"] = serde_json::json!(uuid);
            conn.emit("network:edit", Some(data));
            self.network_modal_open = false;
            self.network_edit_uuid = None;
            self.toast("Network settings saved.");
        } else {
            conn.emit("network:new", Some(data));
            self.network_modal_open = false;
            self.toast("Connecting network...");
        }
        cx.notify();
    }

    pub(super) fn submit_search(&mut self, cx: &mut Context<Self>) {
        let term = self.inputs.search.read(cx).content().trim().to_string();
        if term.is_empty() {
            return;
        }
        let Some(active) = self.state.active else {
            return;
        };
        let Some((network, channel)) = self.state.find_channel(active) else {
            return;
        };
        let Some(conn) = &self.conn else {
            return;
        };
        conn.emit(
            "search",
            Some(serde_json::json!({
                "searchTerm": term,
                "networkUuid": network.uuid,
                "channelName": channel.chan.name,
                "offset": 0,
            })),
        );
    }
}
