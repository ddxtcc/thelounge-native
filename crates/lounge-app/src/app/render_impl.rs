use super::*;

impl Render for LoungeApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(toast) = &self.toast
            && Instant::now() > toast.until
        {
            self.toast = None;
        }

        let overlay_open = self.settings_open
            || self.network_modal_open
            || self.search_open
            || self.mentions_open
            || self.emoji_picker_open
            || self.font_picker.is_some()
            || self.security_open
            || self.network_menu.is_some()
            || self.user_menu.is_some()
            || self.channel_menu.is_some()
            || self.confirm_remove_network.is_some();
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
            if let Some(input) = self.input_for_role(role) {
                let handle = input.read(cx).focus_handle(cx);
                window.focus(&handle);
            }
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
        if self.fonts.is_empty() {
            let mut fonts = window.text_system().all_font_names();
            fonts.retain(|font| !font.is_empty() && !font.contains(','));
            fonts.sort_by_key(|font| font.to_lowercase());
            fonts.dedup();
            self.fonts = fonts;
        }
        let theme = self.theme;
        let mut root = div()
            .id("app-root")
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") && this.last_overlay_open {
                    this.close_overlays(cx);
                    cx.stop_propagation();
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
            || self.mentions_open
            || self.emoji_picker_open
            || self.font_picker.is_some()
            || self.security_open
            || self.network_menu.is_some()
            || self.user_menu.is_some()
            || self.channel_menu.is_some()
            || self.confirm_remove_network.is_some();
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
