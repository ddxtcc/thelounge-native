use super::*;

impl LoungeApp {
    fn update_input_tab_stops(&self, overlay_open: bool, cx: &mut Context<Self>) {
        let entries = [
            (&self.inputs.composer, !overlay_open),
            (&self.inputs.login_server, !overlay_open),
            (&self.inputs.login_user, !overlay_open),
            (&self.inputs.login_password, !overlay_open),
            (
                &self.inputs.network_name,
                !overlay_open || self.network_modal_open,
            ),
            (
                &self.inputs.network_host,
                !overlay_open || self.network_modal_open,
            ),
            (
                &self.inputs.network_port,
                !overlay_open || self.network_modal_open,
            ),
            (
                &self.inputs.network_nick,
                !overlay_open || self.network_modal_open,
            ),
            (
                &self.inputs.network_username,
                !overlay_open || self.network_modal_open,
            ),
            (
                &self.inputs.network_realname,
                !overlay_open || self.network_modal_open,
            ),
            (
                &self.inputs.network_password,
                !overlay_open || self.network_modal_open,
            ),
            (&self.inputs.search, !overlay_open || self.search_open),
            (
                &self.inputs.setting_highlights,
                !overlay_open || self.settings_open,
            ),
            (
                &self.inputs.setting_highlight_exceptions,
                !overlay_open || self.settings_open,
            ),
            (
                &self.inputs.setting_away_message,
                !overlay_open || self.settings_open,
            ),
            (
                &self.inputs.emoji_search,
                !overlay_open || self.emoji_picker_open,
            ),
            (
                &self.inputs.font_search,
                !overlay_open || self.font_picker.is_some(),
            ),
            (&self.inputs.users_filter, !overlay_open),
            (
                &self.inputs.palette,
                !overlay_open || self.palette.is_some(),
            ),
            (
                &self.inputs.password_old,
                !overlay_open || self.security_open,
            ),
            (
                &self.inputs.password_new,
                !overlay_open || self.security_open,
            ),
            (
                &self.inputs.password_verify,
                !overlay_open || self.security_open,
            ),
            (
                &self.inputs.theme_name,
                !overlay_open || self.theme_editor.is_some(),
            ),
            (
                &self.inputs.theme_hex,
                !overlay_open || self.theme_editor.is_some(),
            ),
        ];
        for (input, tab_stop) in entries {
            input.update(cx, |input, _| input.set_tab_stop(tab_stop));
        }
    }
}

impl Render for LoungeApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_show_window {
            self.pending_show_window = false;
            window.activate_window();
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if self.hotkey_manager.is_some() {
            while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
                if event.state == global_hotkey::HotKeyState::Pressed {
                    if window.is_window_active() {
                        window.minimize_window();
                    } else {
                        window.activate_window();
                    }
                }
            }
        }
        let unread = self.state.total_unread();
        let title = if unread > 0 {
            format!("Lounge Native ({unread})")
        } else {
            "Lounge Native".to_string()
        };
        if self.window_title != title {
            window.set_window_title(&title);
            self.window_title = title;
            #[cfg(target_os = "linux")]
            {
                badge::update(unread);
                if let Some(tray) = &self.tray_handle {
                    tray.update(|tray| tray.unread = unread);
                }
            }
            #[cfg(target_os = "windows")]
            if let Some(tray) = &self.tray_handle {
                tray.set_unread(unread);
            }
        }
        let bounds = window.window_bounds().get_bounds();
        let x = f32::from(bounds.origin.x);
        let y = f32::from(bounds.origin.y);
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        if self.config.window_x != Some(x)
            || self.config.window_y != Some(y)
            || self.config.window_width != Some(width)
            || self.config.window_height != Some(height)
        {
            self.config.window_x = Some(x);
            self.config.window_y = Some(y);
            self.config.window_width = Some(width);
            self.config.window_height = Some(height);
            self.window_save_generation = self.window_save_generation.wrapping_add(1);
            let generation = self.window_save_generation;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let _ = this.update(cx, |this, _| {
                    if this.window_save_generation == generation {
                        this.config.save();
                    }
                });
            })
            .detach();
        }
        if let Some(toast) = &self.toast
            && Instant::now() > toast.until
        {
            self.toast = None;
        }

        if let Some(id) = self.pending_channel_switch.take() {
            self.switch_channel_quiet(id, window, cx);
        }

        let overlay_open = self.theme_editor.is_some()
            || self.settings_open
            || self.network_modal_open
            || self.search_open
            || self.shortcuts_open
            || self.mentions_open
            || self.emoji_picker_open
            || self.font_picker.is_some()
            || self.security_open
            || self.network_menu.is_some()
            || self.user_menu.is_some()
            || self.channel_menu.is_some()
            || self.message_menu.is_some()
            || self.confirm_remove_network.is_some()
            || self.lightbox.is_some()
            || self.message_context.is_some()
            || self.palette.is_some();
        self.update_input_tab_stops(overlay_open, cx);
        if overlay_open {
            self.nav_mode = false;
        }
        if overlay_open && !self.last_overlay_open {
            self.overlay_return_focus = window.focused(cx);
            window.focus(&self.root_focus);
        }
        if !overlay_open && self.last_overlay_open {
            if let Some(handle) = self.overlay_return_focus.take() {
                window.focus(&handle);
                self.pending_focus = false;
            } else {
                self.pending_focus = true;
            }
        }
        self.last_overlay_open = overlay_open;

        if let Some(role) = self.focus_request.take() {
            self.pending_focus = false;
            if let Some(input) = self.input_for_role(role) {
                let handle = input.read(cx).focus_handle(cx);
                window.focus(&handle);
            }
        } else if self.focus_root_request {
            self.focus_root_request = false;
            window.focus(&self.root_focus);
        } else if self.pending_focus {
            self.pending_focus = false;
            if matches!(self.route, Route::Chat) {
                self.focus_composer(window, cx);
            } else {
                let handle = self.inputs.login_server.read(cx).focus_handle(cx);
                window.focus(&handle);
            }
        }

        if self.nav_mode && !overlay_open && self.route == Route::Chat {
            self.pending_focus = false;
            self.focus_request = None;
            self.focus_root_request = false;
            window.focus(&self.root_focus);
        }

        self.window_active = window.is_window_active();
        if !self.window_activation_observed {
            self.window_activation_observed = true;
            cx.observe_window_activation(window, |this, window, cx| {
                let active = window.is_window_active();
                if active {
                    if this.deactivated_at.take().is_some_and(|at| {
                        SystemTime::now().duration_since(at).unwrap_or_default()
                            >= Duration::from_secs(5)
                    }) {
                        this.refresh_connection(cx);
                    }
                } else {
                    this.deactivated_at = Some(SystemTime::now());
                }
                this.window_active = active;
            })
            .detach();
        }
        let theme = self.theme;
        let mut root = div()
            .id("app-root")
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.to_lowercase();
                if !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                    && this.handle_navigation_key(&key, event.keystroke.modifiers.shift, window, cx)
                {
                    cx.stop_propagation();
                    return;
                }
                if !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                    && this.handle_overlay_key(&key, event.keystroke.modifiers.shift, window, cx)
                {
                    cx.stop_propagation();
                    return;
                }
                if key == "tab"
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    cx.stop_propagation();
                    return;
                }
                if this.palette == Some(PaletteMode::Hotkey) {
                    let key = event.keystroke.key.to_lowercase();
                    if key == "escape" {
                        this.cancel_hotkey_capture(cx);
                        cx.stop_propagation();
                        return;
                    }
                    let mut parts = Vec::new();
                    if event.keystroke.modifiers.control {
                        parts.push("control");
                    }
                    if event.keystroke.modifiers.alt {
                        parts.push("alt");
                    }
                    if event.keystroke.modifiers.shift {
                        parts.push("shift");
                    }
                    if event.keystroke.modifiers.platform {
                        parts.push("meta");
                    }
                    if parts.is_empty()
                        || matches!(
                            key.as_str(),
                            "control" | "alt" | "shift" | "platform" | "super" | "meta"
                        )
                    {
                        if !matches!(
                            key.as_str(),
                            "control" | "alt" | "shift" | "platform" | "super" | "meta"
                        ) {
                            this.toast("Include Ctrl, Alt, or Meta.");
                        }
                        cx.stop_propagation();
                        return;
                    }
                    parts.push(&key);
                    let spec = parts.join("+");
                    if this.set_global_hotkey(&spec, cx) {
                        this.finish_hotkey_capture(cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                if this.mentions_open
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                {
                    match event.keystroke.key.to_lowercase().as_str() {
                        "up" | "k" => {
                            this.move_mentions(-1, cx);
                            cx.stop_propagation();
                            return;
                        }
                        "down" | "j" => {
                            this.move_mentions(1, cx);
                            cx.stop_propagation();
                            return;
                        }
                        "enter" => {
                            this.activate_mentions_selection(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        _ => {}
                    }
                }
                let secondary =
                    event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
                let chat_ready = this.route == Route::Chat && !this.last_overlay_open;
                if secondary && !event.keystroke.modifiers.alt {
                    let key = event.keystroke.key.to_lowercase();
                    if key == "p" {
                        let mode = if event.keystroke.modifiers.shift {
                            PaletteMode::Commands
                        } else {
                            PaletteMode::Channels
                        };
                        this.open_palette(mode, cx);
                        cx.stop_propagation();
                        return;
                    }
                    if chat_ready {
                        if key == "tab" {
                            this.cycle_channel(
                                if event.keystroke.modifiers.shift {
                                    -1
                                } else {
                                    1
                                },
                                window,
                                cx,
                            );
                            cx.stop_propagation();
                            return;
                        }
                        if key == "pagedown" || key == "pageup" {
                            this.cycle_channel(if key == "pageup" { -1 } else { 1 }, window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if key == "," {
                            this.open_settings(cx);
                            cx.stop_propagation();
                            return;
                        }
                        if key == "f" {
                            this.open_search(cx);
                            cx.stop_propagation();
                            return;
                        }
                        if let Some(digit) = key
                            .chars()
                            .next()
                            .and_then(|character| character.to_digit(10))
                            .filter(|digit| (1..=9).contains(digit))
                        {
                            this.switch_channel_by_index(digit as usize - 1, window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if key == "c" {
                            let focused = window.focused(cx);
                            let composer = this.inputs.composer.read(cx).focus_handle(cx);
                            if focused.as_ref() != Some(&composer) && this.copy_text_selection(cx) {
                                cx.stop_propagation();
                                return;
                            }
                        }
                        if key == "v" && event.keystroke.modifiers.shift {
                            this.paste_image_from_clipboard(cx);
                            cx.stop_propagation();
                            return;
                        }
                        if key == "v" {
                            let focused = window.focused(cx);
                            let composer = this.inputs.composer.read(cx).focus_handle(cx);
                            if focused.as_ref() != Some(&composer)
                                && let Some(text) =
                                    cx.read_from_clipboard().and_then(|item| item.text())
                            {
                                this.inputs
                                    .composer
                                    .update(cx, |input, cx| input.insert_text(&text, cx));
                                this.focus_composer(window, cx);
                                cx.stop_propagation();
                                return;
                            }
                        }
                    }
                }
                if !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                    && this.route == Route::Chat
                    && !this.last_overlay_open
                    && !this.sidebar_overlay
                    && !this.users_overlay
                {
                    let key = event.keystroke.key.to_lowercase();
                    if key == "pageup" {
                        this.scroll_messages_page(1.0, cx);
                        cx.stop_propagation();
                        return;
                    }
                    if key == "pagedown" {
                        this.scroll_messages_page(-1.0, cx);
                        cx.stop_propagation();
                        return;
                    }
                }
                if event.keystroke.key.eq_ignore_ascii_case("escape")
                    && (this.last_overlay_open || this.sidebar_overlay || this.users_overlay)
                {
                    this.close_overlays(cx);
                    cx.stop_propagation();
                    return;
                }
                if this.route == Route::Chat
                    && !this.last_overlay_open
                    && !this.sidebar_overlay
                    && !this.users_overlay
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.function
                    && let Some(text) = event.keystroke.key_char.as_deref()
                    && !text.is_empty()
                    && !text.chars().all(char::is_control)
                {
                    let focused = window.focused(cx);
                    let composer = this.inputs.composer.read(cx).focus_handle(cx);
                    if focused.as_ref() != Some(&composer) {
                        this.inputs
                            .composer
                            .update(cx, |input, cx| input.insert_text(text, cx));
                        this.focus_composer(window, cx);
                        cx.stop_propagation();
                    }
                }
            }))
            .relative()
            .size_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.nav_mode = false;
                }),
            )
            .bg(theme.bg)
            .text_color(theme.text)
            .text_size(px(self.config.font_size))
            .line_height(relative(self.config.line_height))
            .font_family(self.config.font_family.clone())
            .on_drop::<ExternalPaths>(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.upload_paths(paths.paths().to_vec(), cx)
            }));

        match self.route {
            Route::Login => {
                root = root.child(self.render_login(window, cx));
            }
            Route::Chat => {
                root = root.child(self.render_chat(window, cx));
            }
        }

        let overlay_needed = self.settings_open
            || self.network_modal_open
            || self.search_open
            || self.shortcuts_open
            || self.mentions_open
            || self.emoji_picker_open
            || self.font_picker.is_some()
            || self.security_open
            || self.network_menu.is_some()
            || self.user_menu.is_some()
            || self.channel_menu.is_some()
            || self.message_menu.is_some()
            || self.confirm_remove_network.is_some()
            || self.lightbox.is_some()
            || self.message_context.is_some()
            || self.palette.is_some();
        if self.theme_editor.is_some() {
            root = root.child(self.render_theme_editor_window(cx));
        }
        if overlay_needed {
            root = root.child(self.render_modals(window, cx));
        }
        if self.settings_refocus && self.settings_open {
            self.settings_refocus = false;
            window.focus(&self.settings_first_focus);
        }

        if let Some(toast) = &self.toast {
            root = root.child(
                div()
                    .absolute()
                    .bottom(px(84.))
                    .left(px(0.))
                    .right(px(0.))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded(px(6.))
                            .bg(theme.panel)
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(12.))
                            .child(toast.text.clone()),
                    ),
            );
        }

        root
    }
}
