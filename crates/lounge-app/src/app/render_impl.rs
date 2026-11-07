use super::*;

impl Render for LoungeApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while let Ok(channel) = self.notification_action_rx.try_recv() {
            self.pending_channel_switch = Some(channel);
            window.activate_window();
        }
        #[cfg(target_os = "linux")]
        while let Ok(action) = self.tray_action_rx.try_recv() {
            match action {
                tray::TrayAction::Show => window.activate_window(),
                tray::TrayAction::Quit => cx.quit(),
            }
        }
        #[cfg(target_os = "windows")]
        while let Ok(action) = self.tray_action_rx.try_recv() {
            match action {
                tray_windows::TrayAction::Show => window.activate_window(),
                tray_windows::TrayAction::Quit => cx.quit(),
            }
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
        let cache_len = self.image_cache.read(cx).len();
        if cache_len > self.config.image_cache_limit.max(64) {
            let now = Instant::now();
            if self
                .last_image_cache_trim
                .is_none_or(|last| now.duration_since(last) > Duration::from_secs(2))
            {
                self.image_cache
                    .update(cx, |cache, cx| cache.clear(window, cx));
                self.last_image_cache_trim = Some(now);
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
            self.config.save();
        }
        if let Some(toast) = &self.toast
            && Instant::now() > toast.until
        {
            self.toast = None;
        }

        if let Some(id) = self.pending_channel_switch.take() {
            self.switch_channel_quiet(id, window, cx);
        }

        let overlay_open = self.settings_open
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
        self.inputs
            .composer
            .update(cx, |input, _| input.set_tab_stop(!overlay_open));
        if overlay_open && !self.last_overlay_open && self.focus_request.is_none() {
            window.focus(&self.root_focus);
        }
        if !overlay_open && self.last_overlay_open {
            self.pending_focus = true;
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

        self.window_active = window.is_window_active();
        let theme = self.theme;
        let mut root = div()
            .id("app-root")
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
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
                        "up" => {
                            this.move_mentions(-1, cx);
                            cx.stop_propagation();
                            return;
                        }
                        "down" => {
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
                let chat_ready = this.route == Route::Chat
                    && !this.last_overlay_open
                    && !this.sidebar_overlay
                    && !this.users_overlay;
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
        if overlay_needed {
            root = root.child(self.render_modals(window, cx));
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
