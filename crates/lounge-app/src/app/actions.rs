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
            self.inputs.users_filter.clone(),
            self.inputs.palette.clone(),
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
            InputRole::Search
                | InputRole::EmojiSearch
                | InputRole::FontSearch
                | InputRole::UsersFilter
                | InputRole::Palette
        ) {
            if role == InputRole::Palette && self.palette.is_some() {
                self.palette_selection = 0;
                self.palette_scroll.scroll_to_item(0);
            }
            if role == InputRole::Search {
                self.state.search_results = None;
                self.search_selection = 0;
                self.search_scroll.scroll_to_item(0);
            }
            if role == InputRole::EmojiSearch {
                self.emoji_selection = 0;
                self.emoji_scroll.scroll_to_item(0);
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
                InputAction::PasteImage => self.paste_image_from_clipboard(cx),
                InputAction::MoveLeft | InputAction::MoveRight => {}
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
            InputRole::Search => match action {
                InputAction::Submit => {
                    if self
                        .state
                        .search_results
                        .as_ref()
                        .is_some_and(|results| !results.results.is_empty())
                    {
                        self.activate_search_result(self.search_selection, cx);
                    } else {
                        self.submit_search(cx);
                    }
                }
                InputAction::PrevHistory => self.move_search(-1, cx),
                InputAction::NextHistory => self.move_search(1, cx),
                _ => {}
            },
            InputRole::SettingHighlights
            | InputRole::SettingHighlightExceptions
            | InputRole::SettingAwayMessage => {
                if action == InputAction::Submit {
                    self.save_server_setting(role, cx);
                }
            }
            InputRole::UsersFilter => {}
            InputRole::EmojiSearch => match action {
                InputAction::Submit => self.insert_selected_emoji(cx),
                InputAction::PrevHistory => self.move_emoji(-7, cx),
                InputAction::NextHistory => self.move_emoji(7, cx),
                InputAction::MoveLeft => self.move_emoji(-1, cx),
                InputAction::MoveRight => self.move_emoji(1, cx),
                _ => {}
            },
            InputRole::FontSearch => {}
            InputRole::Palette => match action {
                InputAction::Submit => {
                    if self.palette == Some(PaletteMode::Alias) {
                        self.save_command_alias(cx);
                    } else {
                        self.run_palette_selection(cx);
                    }
                }
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
        if !self.status.is_online() {
            self.toast("Not connected. Message left in the composer.");
            return;
        }
        let Some(conn) = &self.conn else {
            self.toast("Not connected. Message left in the composer.");
            return;
        };
        let reply_to = self.reply_to.as_ref().map(|reply| reply.msgid.clone());
        conn.input(target, text.clone(), reply_to);
        conn.emit(
            "typing",
            Some(serde_json::json!({"target": target, "status": "done"})),
        );
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
        self.persist_chat_state();
        self.reply_to = None;
        self.inputs.composer.update(cx, |input, cx| input.clear(cx));
        self.schedule_scroll_bottom(cx);
        cx.notify();
    }

    pub(super) fn send_typing(&mut self, active: bool, cx: &mut Context<Self>) {
        if !self.status.is_online() {
            return;
        }
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
        self.messages_list.reset(0);
        self.messages_list_keys.clear();
        self.message_rows_cache = None;
        self.messages_at_bottom = true;
        self.scroll_to_bottom_pending = false;
        self.image_cache_current.borrow_mut().clear();
        self.image_cache_touched.borrow_mut().clear();
        self.image_cache
            .update(cx, |cache, cx| cache.clear(window, cx));
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
        self.save_draft_for(self.state.active, cx);
    }

    pub(super) fn save_draft_for(&mut self, channel: Option<i64>, cx: &mut Context<Self>) {
        if let Some(active) = channel {
            let text = self.inputs.composer.read(cx).content().to_string();
            if text.trim().is_empty() {
                self.drafts.remove(&active);
            } else {
                self.drafts.insert(active, text);
            }
        }
        self.persist_chat_state();
    }

    pub(super) fn persist_chat_state(&mut self) {
        self.config.saved_drafts = self
            .drafts
            .iter()
            .map(|(channel, text)| crate::config::SavedDraft {
                channel: *channel,
                text: text.clone(),
            })
            .collect();
        self.config.saved_histories = self
            .histories
            .iter()
            .map(|(channel, entries)| crate::config::SavedHistory {
                channel: *channel,
                entries: entries.clone(),
            })
            .collect();
        self.config.save();
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
        let token_start = completion_token_start(&content, trimmed_end);
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

    pub(super) fn channel_image_urls(&self) -> Vec<String> {
        let Some((_, channel)) = self.state.active_channel() else {
            return Vec::new();
        };
        let mut urls = Vec::new();
        for message in &channel.chan.messages {
            for preview in &message.previews {
                let is_image = preview.kind == "image"
                    || preview
                        .media_type
                        .as_deref()
                        .map(|kind| kind.starts_with("image/"))
                        .unwrap_or(false);
                if !is_image {
                    continue;
                }
                let candidate = preview
                    .media
                    .as_deref()
                    .filter(|url| !url.is_empty())
                    .or_else(|| {
                        preview
                            .thumb_actual_url
                            .as_deref()
                            .filter(|url| !url.is_empty())
                    })
                    .or_else(|| Some(preview.thumb.as_str()).filter(|url| !url.is_empty()))
                    .unwrap_or(&preview.link);
                urls.push(self.absolute_media_url(candidate));
            }
            for link in format::find_links(&message.text) {
                let path = link
                    .url
                    .split(['?', '#'])
                    .next()
                    .unwrap_or(&link.url)
                    .to_lowercase();
                if [".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".bmp"]
                    .iter()
                    .any(|extension| path.ends_with(extension))
                {
                    urls.push(self.absolute_media_url(&link.url));
                }
            }
        }
        let mut seen = HashSet::new();
        urls.retain(|url| seen.insert(url.clone()));
        urls
    }

    pub(super) fn touch_image(&self, url: &str) {
        let url = url.to_string();
        self.image_cache_current.borrow_mut().insert(url.clone());
        let mut touched = self.image_cache_touched.borrow_mut();
        if let Some(position) = touched.iter().position(|candidate| candidate == &url) {
            touched.remove(position);
        }
        touched.push_back(url);
    }

    pub(super) fn trim_image_cache(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let limit = self.config.image_cache_limit.max(64);
        let current = self.image_cache_current.borrow().clone();
        let mut touched = self.image_cache_touched.borrow_mut();
        while touched.len() > limit {
            let Some(position) = image_eviction_position(&touched, &current) else {
                break;
            };
            let Some(url) = touched.remove(position) else {
                break;
            };
            let resource = gpui::Resource::Uri(url.into());
            self.image_cache
                .update(cx, |cache, cx| cache.remove(&resource, window, cx));
        }
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
        self.scroll_to_bottom_pending = true;
        self.messages_at_bottom = true;
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
                if let Some(range) = selection.range_for(channel, message.id, &text)
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

    pub(super) fn scroll_messages_by(&mut self, delta: Pixels, cx: &mut Context<Self>) {
        self.messages_list.scroll_by(-delta);
        self.refresh_at_bottom();
        cx.notify();
    }

    pub(super) fn scroll_messages_page(&mut self, direction: f32, cx: &mut Context<Self>) {
        let height = self
            .messages_bounds
            .map(|bounds| bounds.size.height)
            .unwrap_or(px(600.));
        self.messages_list.scroll_by(-height * direction);
        self.refresh_at_bottom();
        cx.notify();
    }

    fn refresh_at_bottom(&mut self) {
        let top = self.messages_list.logical_scroll_top();
        self.messages_at_bottom = top.item_ix + 3 >= self.messages_list.item_count();
    }

    pub(super) fn at_bottom(&self) -> bool {
        self.messages_at_bottom
    }

    pub(super) fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            until: Instant::now() + Duration::from_secs(6),
        });
    }

    pub(super) fn play_media_or_open(&mut self, url: String, cx: &mut Context<Self>) {
        #[cfg(feature = "media-video")]
        {
            if super::media::media_kind(&url).is_some() {
                self.open_media_lightbox(url, cx);
                return;
            }
        }
        #[cfg(not(feature = "media-video"))]
        let _ = cx;
        let _ = open::that(url);
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
        let mut data = self.network_edit_data.take().unwrap_or_default();
        data.insert("name".into(), serde_json::json!(name));
        data.insert("nick".into(), serde_json::json!(nick));
        data.insert("username".into(), serde_json::json!(username));
        data.insert("realname".into(), serde_json::json!(realname));
        data.insert("password".into(), serde_json::json!(password));
        data.insert("tls".into(), serde_json::json!(self.network_tls));
        data.entry("rejectUnauthorized")
            .or_insert_with(|| serde_json::json!(true));
        if !host.is_empty() {
            data.insert("host".into(), serde_json::json!(host));
            if let Ok(port) = port_text.parse::<u16>() {
                data.insert("port".into(), serde_json::json!(port));
            }
        }
        if self.network_edit_loading {
            self.toast("Loading network settings...");
            return;
        }
        if let Some(uuid) = self.network_edit_uuid.clone() {
            data.insert("uuid".into(), serde_json::json!(uuid));
            conn.emit("network:edit", Some(serde_json::Value::Object(data)));
            self.network_modal_open = false;
            self.network_edit_uuid = None;
            self.toast("Network settings saved.");
        } else {
            conn.emit("network:new", Some(serde_json::Value::Object(data)));
            self.network_modal_open = false;
            self.toast("Connecting network...");
        }
        cx.notify();
    }
}

fn completion_token_start(content: &str, end: usize) -> usize {
    content[..end]
        .char_indices()
        .rev()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index + ch.len_utf8()))
        .unwrap_or(0)
}

fn image_eviction_position(touched: &VecDeque<String>, current: &HashSet<String>) -> Option<usize> {
    touched.iter().position(|url| !current.contains(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_eviction_skips_currently_rendered_images() {
        let touched = VecDeque::from([
            "old".to_string(),
            "visible".to_string(),
            "older".to_string(),
        ]);
        let current = HashSet::from(["visible".to_string()]);
        assert_eq!(image_eviction_position(&touched, &current), Some(0));
    }

    #[test]
    fn image_eviction_returns_none_when_all_images_are_visible() {
        let touched = VecDeque::from(["one".to_string(), "two".to_string()]);
        let current = HashSet::from(["one".to_string(), "two".to_string()]);
        assert_eq!(image_eviction_position(&touched, &current), None);
    }

    #[test]
    fn completion_token_start_handles_multibyte_whitespace() {
        assert_eq!(completion_token_start("foo bar", 7), 4);
        assert_eq!(completion_token_start("foo\u{00a0}bar", 8), 5);
        assert_eq!(completion_token_start("foo\u{3000}bar", 8), 6);
    }
}
