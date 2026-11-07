use super::*;

impl LoungeApp {
    pub(super) fn attempt_login(&mut self, cx: &mut Context<Self>) {
        let server = self
            .inputs
            .login_server
            .read(cx)
            .content()
            .trim()
            .to_string();
        let user = self.inputs.login_user.read(cx).content().trim().to_string();
        let password = self.inputs.login_password.read(cx).content().to_string();

        if server.is_empty() {
            self.login_error = Some("Server URL is required.".into());
            self.pending_focus = true;
            cx.notify();
            return;
        }

        let server = if server.starts_with("http://") || server.starts_with("https://") {
            server
        } else {
            format!("http://{server}")
        };
        if lounge_core::proto::socket_url(&server).is_err() {
            self.login_error = Some("That is not a valid server URL.".into());
            cx.notify();
            return;
        }

        if server != self.config.server {
            self.config.token.clear();
            self.config.last_channel = None;
        }
        if user != self.config.user {
            self.config.token.clear();
        }
        self.config.server = server.clone();
        self.config.user = user.clone();
        self.config.save();

        self.login_error = None;
        let auth = if user.is_empty() && password.is_empty() {
            Auth::Public
        } else {
            Auth::Password { user, password }
        };
        self.start_connection(auth, server, cx);
    }

    pub(super) fn start_connection(&mut self, auth: Auth, server: String, cx: &mut Context<Self>) {
        if let Some(conn) = self.conn.take() {
            conn.close();
        }
        self.conn_generation = self.conn_generation.wrapping_add(1);
        let generation = self.conn_generation;
        self.state = ClientState::new();
        self.messages_list.reset(0);
        self.messages_list_keys.clear();
        self.message_rows_cache = None;
        self.messages_at_bottom = true;
        self.scroll_to_bottom_pending = false;
        self.reset_uploads();
        self.reply_to = None;
        self.history_index = 0;
        self.typing_active = false;
        self.status = ConnectionStatus::Connecting;
        self.login_error = None;
        self.last_active_channel = None;
        self.last_message_id = 0;

        let (handle, mut events) = lounge_core::socket::connect(ClientConfig {
            base_url: server,
            auth,
            initial_last_message: -1,
            initial_open_channel: self.config.last_channel,
            has_config: false,
        });
        self.conn = Some(handle);
        cx.spawn(async move |this, cx| {
            while let Some(event) = events.recv().await {
                let keep_going = this
                    .update(cx, |this, cx| {
                        this.handle_client_event(generation, event, cx)
                    })
                    .unwrap_or(false);
                if !keep_going {
                    break;
                }
            }
        })
        .detach();
        self.pending_focus = true;
        cx.notify();
    }

    pub(super) fn handle_client_event(
        &mut self,
        generation: u64,
        event: ClientEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if generation != self.conn_generation {
            return false;
        }
        match event {
            ClientEvent::Connecting => {
                self.status = ConnectionStatus::Connecting;
            }
            ClientEvent::Handshake => {
                self.status = ConnectionStatus::Authenticating;
            }
            ClientEvent::Authenticating => {
                self.status = ConnectionStatus::Authenticating;
            }
            ClientEvent::Authenticated => {}
            ClientEvent::AuthFailed => {
                self.status = ConnectionStatus::Disconnected;
                self.config.token.clear();
                self.config.save();
                self.route = Route::Login;
                self.login_error = Some("Authentication failed. Check your credentials.".into());
                self.pending_focus = true;
            }
            ClientEvent::Init(init) => {
                let old_active = self.state.active;
                self.state.apply(ServerEvent::Init(init));
                self.route = Route::Chat;
                self.status = ConnectionStatus::Online;
                self.login_error = None;
                self.after_init(old_active, cx);
            }
            ClientEvent::Server(event) => {
                let event = *event;
                let old_active = self.state.active;
                let is_more = matches!(event, ServerEvent::More(_));
                let signed_out = matches!(event, ServerEvent::SignedOut);
                let is_message = matches!(event, ServerEvent::Message(_));
                let is_search_results = matches!(event, ServerEvent::SearchResults(_));
                let is_mentions = matches!(event, ServerEvent::Mentions(_));
                let upload_token = match &event {
                    ServerEvent::UploadAuth(token) => Some(token.clone()),
                    _ => None,
                };
                let network_info = match &event {
                    ServerEvent::NetworkInfo(info) => Some(info.clone()),
                    _ => None,
                };
                let server_error = match &event {
                    ServerEvent::Error(value) => Some(match value {
                        serde_json::Value::String(text) => text.clone(),
                        other => other.to_string(),
                    }),
                    _ => None,
                };
                let password_result = match &event {
                    ServerEvent::ChangePassword(result) => Some(result.clone()),
                    _ => None,
                };
                if let ServerEvent::Message(message) = &event
                    && message.msg.highlight
                    && self.config.notifications
                    && !self.window_active
                    && self.state.active != Some(message.chan)
                {
                    let nick = message
                        .msg
                        .from
                        .as_ref()
                        .map(|from| from.nick.clone())
                        .unwrap_or_default();
                    let channel_name = self
                        .state
                        .find_channel(message.chan)
                        .map(|(_, channel)| channel.chan.name.clone())
                        .unwrap_or_default();
                    let body = format::clean(&message.msg.text);
                    let chan_id = message.chan;
                    #[cfg(target_os = "linux")]
                    {
                        let actions = self.notification_action_tx.clone();
                        std::thread::spawn(move || {
                            let output = std::process::Command::new("notify-send")
                                .arg("--wait")
                                .arg("--expire-time=10000")
                                .arg("--action=default=Open")
                                .arg("--app-name=Lounge Native")
                                .arg(format!("{nick} ({channel_name})"))
                                .arg(body)
                                .output();
                            if let Ok(output) = output
                                && String::from_utf8_lossy(&output.stdout).contains("default")
                            {
                                let _ = actions.send(chan_id);
                            }
                        });
                    }
                    #[cfg(target_os = "windows")]
                    {
                        let actions = self.notification_action_tx.clone();
                        let title = format!("{nick} ({channel_name})");
                        std::thread::spawn(move || {
                            use tauri_winrt_notification::Toast;
                            let _ = Toast::new("Lounge.Native")
                                .title(&title)
                                .text1(&body)
                                .on_activated(move |_| {
                                    let _ = actions.send(chan_id);
                                    Ok(())
                                })
                                .show();
                        });
                    }
                    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                    {
                        let _ = (body, chan_id, nick, channel_name);
                    }
                }
                self.state.apply(event);
                if is_search_results {
                    self.search_selection = 0;
                    self.search_scroll.scroll_to_item(0);
                }
                if is_mentions {
                    if self.mentions_open {
                        self.mark_mentions_seen();
                    }
                    self.mentions_selection = 0;
                    self.mentions_scroll.scroll_to_item(0);
                }
                if let Some(token) = upload_token {
                    self.upload_next(token, cx);
                }
                if let Some(info) = network_info {
                    self.prefill_network_edit(&info, cx);
                }
                if signed_out {
                    self.config.token.clear();
                    self.config.save();
                    self.route = Route::Login;
                    self.status = ConnectionStatus::Disconnected;
                    self.login_error = Some("Signed out.".into());
                    self.conn_generation = self.conn_generation.wrapping_add(1);
                    if let Some(conn) = self.conn.take() {
                        conn.close();
                    }
                    self.pending_focus = true;
                    cx.notify();
                    return false;
                }
                if let Some(error) = server_error {
                    self.toast(error);
                }
                if let Some(result) = password_result {
                    if result.success {
                        self.inputs
                            .password_old
                            .update(cx, |input, cx| input.clear(cx));
                        self.inputs
                            .password_new
                            .update(cx, |input, cx| input.clear(cx));
                        self.inputs
                            .password_verify
                            .update(cx, |input, cx| input.clear(cx));
                        self.toast("Password changed.");
                    } else {
                        self.toast("Password change failed. Check your current password.");
                    }
                }
                self.after_state_event(old_active, is_more, is_message, cx);
            }
            ClientEvent::Disconnected { reason } => {
                self.status = ConnectionStatus::Reconnecting(format!("Disconnected: {reason}"));
            }
            ClientEvent::Reconnecting { attempt, delay } => {
                self.status = ConnectionStatus::Reconnecting(format!(
                    "Reconnecting in {}s (attempt {attempt})...",
                    delay.as_secs()
                ));
            }
            ClientEvent::Error(message) => {
                self.status = ConnectionStatus::Failed(message.clone());
                self.toast(message);
            }
            ClientEvent::Closed => {
                if matches!(self.route, Route::Chat) {
                    self.status = ConnectionStatus::Disconnected;
                }
                return false;
            }
        }
        cx.notify();
        true
    }

    pub(super) fn after_init(&mut self, old_active: Option<i64>, cx: &mut Context<Self>) {
        if let Some(token) = self.state.token.clone() {
            self.config.token = token;
        }
        if let Some(active) = self.state.active {
            self.config.last_channel = Some(active);
        }
        self.config.save();

        self.schedule_scroll_bottom(cx);
        self.last_active_channel = self.state.active;
        self.last_message_id = self
            .state
            .active_channel()
            .and_then(|(_, channel)| channel.chan.messages.last().map(|m| m.id))
            .unwrap_or(0);
        if let (Some(conn), Some(active)) = (&self.conn, self.state.active) {
            conn.open(active);
            conn.emit("setting:get", None);
        }
        self.open_active_channel(cx, true);
        if old_active != self.state.active {
            self.restore_draft(cx);
        }
        self.pending_focus = true;
    }

    pub(super) fn after_state_event(
        &mut self,
        old_active: Option<i64>,
        is_more: bool,
        is_message: bool,
        cx: &mut Context<Self>,
    ) {
        if is_more && self.scroll_after_more {
            self.scroll_after_more = false;
            self.schedule_scroll_bottom(cx);
        }
        if old_active != self.state.active {
            self.save_draft_for(old_active, cx);
            self.last_active_channel = self.state.active;
            self.config.last_channel = self.state.active;
            self.config.save();
            if let (Some(conn), Some(active)) = (&self.conn, self.state.active) {
                conn.open(active);
            }
            self.open_active_channel(cx, true);
            self.restore_draft(cx);
            self.schedule_scroll_bottom(cx);
            self.pending_focus = true;
        } else if !is_more {
            self.open_active_channel(cx, false);
            let new_last = self
                .state
                .active_channel()
                .and_then(|(_, channel)| channel.chan.messages.last().map(|m| m.id))
                .unwrap_or(self.last_message_id);
            if is_message && new_last > self.last_message_id {
                let at_bottom = self.at_bottom();
                let from_self = self
                    .state
                    .active_channel()
                    .and_then(|(_, channel)| channel.chan.messages.last())
                    .map(|message| message.is_self)
                    .unwrap_or(false);
                if at_bottom || from_self {
                    self.schedule_scroll_bottom(cx);
                }
            }
            self.last_message_id = new_last;
        }
        self.last_known_revision = self.state.revision;
    }

    pub(super) fn open_active_channel(&mut self, cx: &mut Context<Self>, allow_history: bool) {
        let Some(id) = self.state.active else {
            return;
        };
        let (needs_names, can_auto_history, already_loading) = self
            .state
            .find_channel(id)
            .map(|(_, channel)| {
                (
                    channel.users_outdated,
                    channel.chan.messages.len() <= 1 && channel.more_history_available(),
                    channel.history_loading,
                )
            })
            .unwrap_or((false, false, false));
        if needs_names {
            if let Some(channel) = self.state.find_channel_mut(id) {
                channel.users_outdated = false;
            }
            if let Some(conn) = &self.conn {
                conn.names(id);
            }
        }
        if allow_history && can_auto_history && !already_loading {
            self.scroll_after_more = true;
            if let Some(last_id) = self.state.mark_history_requested(id)
                && let Some(conn) = &self.conn
            {
                conn.more(
                    id,
                    last_id,
                    self.config.status_messages != StatusMessages::Shown,
                );
            }
        }
        self.update_composer_placeholder(cx);
    }

    pub(super) fn update_composer_placeholder(&mut self, cx: &mut Context<Self>) {
        let placeholder = self
            .state
            .active_channel()
            .map(|(_, channel)| match channel.chan.kind {
                lounge_core::ChanType::Query => format!("Message @{}...", channel.chan.name),
                _ => format!("Message {}...", channel.chan.name),
            })
            .unwrap_or_else(|| "Message...".into());
        self.inputs.composer.update(cx, |input, cx| {
            input.set_placeholder(placeholder);
            cx.notify();
        });
    }

    pub(super) fn sign_out(&mut self, cx: &mut Context<Self>) {
        if let Some(conn) = &self.conn {
            conn.emit("sign-out", None);
            conn.close();
        }
        self.conn_generation = self.conn_generation.wrapping_add(1);
        self.conn = None;
        self.reset_uploads();
        self.config.token.clear();
        self.config.save();
        self.route = Route::Login;
        self.status = ConnectionStatus::Disconnected;
        self.login_error = Some("Signed out.".into());
        self.pending_focus = true;
        cx.notify();
    }
}
