use super::*;

fn settings_activation(event: &gpui::KeyDownEvent) -> bool {
    let key = event.keystroke.key.to_lowercase();
    matches!(key.as_str(), "enter" | "return" | "space" | " ")
        && !event.keystroke.modifiers.control
        && !event.keystroke.modifiers.alt
        && !event.keystroke.modifiers.platform
}

impl LoungeApp {
    pub(super) fn render_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let viewport_height = window.viewport_size().height;
        let height = (viewport_height - px(80.)).clamp(px(360.), px(620.));
        let mut card = self.modal_card("Settings", cx).fixed_height(height);
        card = card.child(self.settings_tabs(cx));
        let content = div().flex().flex_col().child(
            div()
                .track_focus(&self.settings_first_focus)
                .tab_index(0)
                .tab_stop(false)
                .h(px(0.)),
        );
        let content = match self.settings_tab {
            SettingsTab::Appearance => content.child(self.settings_appearance(cx)),
            SettingsTab::Messages => content.child(self.settings_messages(cx)),
            SettingsTab::General => content.child(self.settings_general(cx)),
            SettingsTab::Server => content.child(self.settings_server(cx)),
            SettingsTab::About => content.child(self.settings_about(cx)),
        };
        card.child(content).into_any_element()
    }

    fn settings_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut tabs = div().flex().flex_wrap().gap_1().pb_2();
        for (tab, label) in [
            (SettingsTab::Appearance, "Appearance"),
            (SettingsTab::Messages, "Messages"),
            (SettingsTab::General, "General"),
            (SettingsTab::Server, "Server"),
            (SettingsTab::About, "About"),
        ] {
            let active = self.settings_tab == tab;
            tabs = tabs.child(
                div()
                    .id(ElementId::Name(format!("settings-tab-{label}").into()))
                    .px_3()
                    .py_1()
                    .rounded(px(5.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(active, |this| {
                        this.bg(theme.accent_dim).text_color(theme.accent)
                    })
                    .when(!active, |this| {
                        this.text_color(theme.muted)
                            .hover(|style| style.bg(theme.hover))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings_tab = tab;
                        this.settings_refocus = true;
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        tabs.into_any_element()
    }

    fn settings_appearance(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let card = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.setting_section("Theme"))
            .child(self.theme_picker(cx))
            .child(
                self.setting_row_button("New custom theme", "Create", cx, |this, cx| {
                    this.new_theme_editor(cx)
                }),
            )
            .child(
                self.setting_row_button("Edit custom theme", "Edit", cx, |this, cx| {
                    this.edit_selected_theme(cx)
                }),
            )
            .child(
                self.setting_row_button("Import custom theme", "Paste", cx, |this, cx| {
                    this.import_theme(cx)
                }),
            )
            .child(
                self.setting_row_button("Export current theme", "Copy", cx, |this, cx| {
                    this.export_theme(cx)
                }),
            )
            .child(self.setting_section("Typography"))
            .child(self.setting_row_action(
                "Font size",
                &format!("{:.0}px", self.config.font_size),
                cx,
                |this, cx| {
                    this.config.font_size = (this.config.font_size - 1.0).max(11.0);
                    this.apply_theme(cx);
                    this.config.save();
                },
                |this, cx| {
                    this.config.font_size = (this.config.font_size + 1.0).min(24.0);
                    this.apply_theme(cx);
                    this.config.save();
                },
            ))
            .child(self.setting_row_value_button(
                "Message font",
                if self.config.font_family == ".SystemUIFont" {
                    "System UI"
                } else {
                    &self.config.font_family
                },
                "Choose",
                cx,
                |this, _, cx| this.open_font_picker(FontTarget::Ui, cx),
            ))
            .child(self.setting_row_value_button(
                "Monospace font",
                if self.config.monospace_font == "monospace" {
                    "Monospace"
                } else {
                    &self.config.monospace_font
                },
                "Choose",
                cx,
                |this, _, cx| this.open_font_picker(FontTarget::Mono, cx),
            ))
            .child(self.setting_row_action(
                "Line height",
                &format!("{:.2}x", self.config.line_height),
                cx,
                |this, cx| {
                    this.config.line_height = (this.config.line_height - 0.05).max(1.0);
                    this.apply_theme(cx);
                    this.config.save();
                },
                |this, cx| {
                    this.config.line_height = (this.config.line_height + 0.05).min(2.2);
                    this.apply_theme(cx);
                    this.config.save();
                },
            ))
            .child(self.setting_section("Display"))
            .child(self.setting_row_toggle(
                "Show timestamps",
                self.config.show_timestamps,
                cx,
                |this| this.config.show_timestamps = !this.config.show_timestamps,
            ))
            .child(self.setting_row_toggle(
                "24-hour clock",
                !self.config.use_12h_clock,
                cx,
                |this| this.config.use_12h_clock = !this.config.use_12h_clock,
            ))
            .child(
                self.setting_row_toggle("Show seconds", self.config.show_seconds, cx, |this| {
                    this.config.show_seconds = !this.config.show_seconds
                }),
            )
            .child(self.setting_row_toggle(
                "Colored nicknames",
                self.config.colored_nicks,
                cx,
                |this| this.config.colored_nicks = !this.config.colored_nicks,
            ))
            .child(self.setting_row_toggle(
                "Show user modes (@, +)",
                self.config.show_user_modes,
                cx,
                |this| this.config.show_user_modes = !this.config.show_user_modes,
            ))
            .child(
                self.setting_row_toggle("Icon buttons", self.config.icon_buttons, cx, |this| {
                    this.config.icon_buttons = !this.config.icon_buttons
                }),
            )
            .child(self.setting_row_cycle(
                "Image cache limit",
                &format!("{} images", self.config.image_cache_limit),
                cx,
                |this| {
                    this.config.image_cache_limit = match this.config.image_cache_limit {
                        0..=128 => 256,
                        129..=256 => 512,
                        257..=512 => 1024,
                        _ => 128,
                    };
                },
            ));
        card.into_any_element()
    }

    fn settings_messages(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let card = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.setting_section("Messages"))
            .child(self.setting_row_action(
                "Message spacing",
                &format!("{:.0}px", self.config.message_spacing),
                cx,
                |this, cx| {
                    this.config.message_spacing = (this.config.message_spacing - 1.0).max(0.0);
                    this.config.save();
                    cx.notify();
                },
                |this, cx| {
                    this.config.message_spacing = (this.config.message_spacing + 1.0).min(8.0);
                    this.config.save();
                    cx.notify();
                },
            ))
            .child(self.setting_row_toggle(
                "Aligned message text",
                self.config.nick_column,
                cx,
                |this| this.config.nick_column = !this.config.nick_column,
            ))
            .child(self.setting_section("Features"))
            .child(self.setting_row_toggle(
                "Unread jump button",
                self.config.features.unread_jump,
                cx,
                |this| this.config.features.unread_jump = !this.config.features.unread_jump,
            ))
            .child(self.setting_row_toggle(
                "Recent emoji",
                self.config.features.recent_emoji,
                cx,
                |this| this.config.features.recent_emoji = !this.config.features.recent_emoji,
            ))
            .child(self.setting_row_toggle(
                "Pinned messages",
                self.config.features.pinned_messages,
                cx,
                |this| this.config.features.pinned_messages = !this.config.features.pinned_messages,
            ))
            .child(self.setting_row_toggle(
                "Emoji suggestions while typing",
                self.config.features.emoji_suggestions,
                cx,
                |this| {
                    this.config.features.emoji_suggestions = !this.config.features.emoji_suggestions
                },
            ))
            .child(self.setting_row_toggle(
                "Click reply to jump",
                self.config.features.reply_jump,
                cx,
                |this| this.config.features.reply_jump = !this.config.features.reply_jump,
            ))
            .child(self.setting_row_toggle(
                "Typing indicators",
                self.config.features.typing_indicators,
                cx,
                |this| {
                    this.config.features.typing_indicators = !this.config.features.typing_indicators
                },
            ))
            .child(self.setting_row_toggle(
                "Clickable timestamps",
                self.config.features.timestamp_links,
                cx,
                |this| this.config.features.timestamp_links = !this.config.features.timestamp_links,
            ))
            .child(self.setting_row_toggle(
                "Paste images from clipboard",
                self.config.features.clipboard_images,
                cx,
                |this| {
                    this.config.features.clipboard_images = !this.config.features.clipboard_images
                },
            ))
            .child(self.setting_row_toggle(
                "Video playback speed",
                self.config.features.video_speed,
                cx,
                |this| this.config.features.video_speed = !this.config.features.video_speed,
            ))
            .child(self.setting_row_toggle(
                "Large emoji-only messages",
                self.config.features.large_emoji,
                cx,
                |this| this.config.features.large_emoji = !this.config.features.large_emoji,
            ))
            .child(self.setting_row_toggle(
                "Channel List filter and sort",
                self.config.features.channel_list_tools,
                cx,
                |this| {
                    this.config.features.channel_list_tools =
                        !this.config.features.channel_list_tools
                },
            ))
            .child(self.setting_row_toggle(
                "Draft indicators",
                self.config.features.draft_indicators,
                cx,
                |this| {
                    this.config.features.draft_indicators = !this.config.features.draft_indicators
                },
            ))
            .child(self.setting_row_toggle(
                "Hide muted channels",
                self.config.features.hide_muted_channels,
                cx,
                |this| {
                    this.config.features.hide_muted_channels =
                        !this.config.features.hide_muted_channels
                },
            ))
            .child(self.setting_row_toggle(
                "Channel topic tooltips",
                self.config.features.channel_topic_tooltips,
                cx,
                |this| {
                    this.config.features.channel_topic_tooltips =
                        !this.config.features.channel_topic_tooltips
                },
            ))
            .child(self.setting_row_toggle(
                "Full timestamp tooltips",
                self.config.features.timestamp_tooltips,
                cx,
                |this| {
                    this.config.features.timestamp_tooltips =
                        !this.config.features.timestamp_tooltips
                },
            ))
            .child(self.setting_row_toggle(
                "Wheel switches channels",
                self.config.features.wheel_channel_switch,
                cx,
                |this| {
                    this.config.features.wheel_channel_switch =
                        !this.config.features.wheel_channel_switch
                },
            ))
            .child(self.setting_row_toggle(
                "Pin markers on messages",
                self.config.features.pin_markers,
                cx,
                |this| this.config.features.pin_markers = !this.config.features.pin_markers,
            ))
            .child(self.setting_row_toggle(
                "Hide date separators",
                self.config.features.hide_date_separators,
                cx,
                |this| {
                    this.config.features.hide_date_separators =
                        !this.config.features.hide_date_separators
                },
            ))
            .child(self.setting_row_toggle(
                "Dim own messages",
                self.config.features.dim_own_messages,
                cx,
                |this| {
                    this.config.features.dim_own_messages = !this.config.features.dim_own_messages
                },
            ))
            .child(self.setting_row_cycle(
                "Status messages",
                match self.config.status_messages {
                    StatusMessages::Condensed => "Condensed",
                    StatusMessages::Shown => "Shown",
                    StatusMessages::Hidden => "Hidden",
                },
                cx,
                |this| {
                    this.config.status_messages = match this.config.status_messages {
                        StatusMessages::Condensed => StatusMessages::Shown,
                        StatusMessages::Shown => StatusMessages::Hidden,
                        StatusMessages::Hidden => StatusMessages::Condensed,
                    };
                },
            ))
            .child(self.setting_row_cycle(
                "Image and media previews",
                match self.config.preview_mode {
                    crate::config::PreviewMode::Shown => "Shown",
                    crate::config::PreviewMode::Collapsed => "Hidden until expanded",
                    crate::config::PreviewMode::Hidden => "Hidden",
                },
                cx,
                |this| {
                    this.config.preview_mode = match this.config.preview_mode {
                        crate::config::PreviewMode::Shown => crate::config::PreviewMode::Collapsed,
                        crate::config::PreviewMode::Collapsed => crate::config::PreviewMode::Hidden,
                        crate::config::PreviewMode::Hidden => crate::config::PreviewMode::Shown,
                    };
                    this.config.show_inline_images =
                        this.config.preview_mode != crate::config::PreviewMode::Hidden;
                },
            ))
            .child(self.setting_row_toggle(
                "Show link previews",
                self.config.show_previews,
                cx,
                |this| this.config.show_previews = !this.config.show_previews,
            ))
            .child(self.setting_row_toggle(
                "Generate video thumbnails",
                self.config.video_thumbnails,
                cx,
                |this| this.config.video_thumbnails = !this.config.video_thumbnails,
            ))
            .child(self.setting_row_toggle(
                "Remember playback positions",
                self.config.remember_media_positions,
                cx,
                |this| {
                    this.config.remember_media_positions = !this.config.remember_media_positions;
                    if !this.config.remember_media_positions {
                        this.config.media_positions.clear();
                    }
                },
            ))
            .child(self.setting_row_button(
                "Clear current channel history",
                "Clear",
                cx,
                |this, cx| {
                    if let Some(target) = this.state.active
                        && let Some(conn) = &this.conn
                    {
                        conn.emit("history:clear", Some(serde_json::json!({"target": target})));
                        this.toast("Cleared channel history.");
                    }
                    cx.notify();
                },
            ));
        card.into_any_element()
    }

    fn settings_general(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut card = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.setting_section("Interface"))
            .child(self.setting_row_toggle(
                "Show user list",
                self.config.show_user_list,
                cx,
                |this| this.config.show_user_list = !this.config.show_user_list,
            ))
            .child(self.setting_row_toggle(
                "Show network sidebar",
                self.config.show_sidebar,
                cx,
                |this| this.config.show_sidebar = !this.config.show_sidebar,
            ))
            .child(self.setting_row_toggle(
                "Sort users by mode",
                self.config.sort_users_by_mode,
                cx,
                |this| this.config.sort_users_by_mode = !this.config.sort_users_by_mode,
            ))
            .child(self.setting_section("Notifications"))
            .child(self.setting_row_toggle(
                "Desktop notifications",
                self.config.notifications,
                cx,
                |this| this.config.notifications = !this.config.notifications,
            ))
            .child(self.setting_row_cycle(
                "Notification mode",
                self.config.notification_mode.label(),
                cx,
                |this| this.config.notification_mode = this.config.notification_mode.next(),
            ));
        if let Some((_, channel)) = self.state.active_channel() {
            let channel_id = channel.chan.id;
            let channel_name = channel.chan.name.clone();
            let override_mode = self.config.channel_notification_override(channel_id);
            let value = override_mode
                .map(|mode| mode.label().to_string())
                .unwrap_or_else(|| format!("Global ({})", self.config.notification_mode.label()));
            card = card.child(self.setting_row_cycle(
                &format!("{channel_name} notifications"),
                &value,
                cx,
                move |this| {
                    let next = match this.config.channel_notification_override(channel_id) {
                        None => Some(NotificationMode::Mentions),
                        Some(NotificationMode::Mentions) => Some(NotificationMode::All),
                        Some(NotificationMode::All) => Some(NotificationMode::DirectMessages),
                        Some(NotificationMode::DirectMessages) => None,
                    };
                    this.config.set_channel_notification_mode(channel_id, next);
                },
            ));
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            card = card
                .child(self.setting_section("Desktop"))
                .child(self.setting_row_value_button(
                    "Show tray icon",
                    if self.config.show_tray_icon {
                        "On"
                    } else {
                        "Off"
                    },
                    "Toggle",
                    cx,
                    |this, _, cx| {
                        this.set_tray_enabled(!this.config.show_tray_icon, cx);
                    },
                ))
                .child(self.setting_row_value_button(
                    "Global shortcut",
                    &self.config.global_hotkey,
                    "Change",
                    cx,
                    |this, _, cx| this.open_hotkey_palette(true, cx),
                ))
                .child(self.setting_row_value_button(
                    "Global shortcut enabled",
                    if self.config.enable_global_hotkey {
                        "On"
                    } else {
                        "Off"
                    },
                    "Toggle",
                    cx,
                    |this, _, cx| {
                        this.set_global_hotkey_enabled(!this.config.enable_global_hotkey, cx);
                    },
                ));
        }
        card.into_any_element()
    }

    fn settings_server(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let card = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.setting_section("Account"))
            .child(
                self.setting_row_button("Security & sessions", "Open", cx, |this, cx| {
                    this.settings_open = false;
                    this.security_open = true;
                    if let Some(conn) = &this.conn {
                        conn.emit("sessions:get", None);
                    }
                    cx.notify();
                }),
            )
            .child(self.setting_section("Server settings"))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Press Enter in a field to save it on the server."),
            )
            .child(self.labeled_input("Highlight words", &self.inputs.setting_highlights))
            .child(self.labeled_input(
                "Highlight exceptions",
                &self.inputs.setting_highlight_exceptions,
            ))
            .child(self.labeled_input("Away message", &self.inputs.setting_away_message));
        card.into_any_element()
    }

    fn settings_about(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut card = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted)
                    .child(format!("Lounge Native {}", env!("CARGO_PKG_VERSION"))),
            )
            .child(
                div().text_size(px(12.)).text_color(theme.muted).child(
                    self.state
                        .configuration
                        .as_ref()
                        .map(|config| {
                            format!(
                                "The Lounge server {} - connected to {}",
                                if config.version.is_empty() {
                                    "unknown version"
                                } else {
                                    &config.version
                                },
                                self.config.server
                            )
                        })
                        .unwrap_or_else(|| "Not connected".into()),
                ),
            )
            .child(self.setting_row_button(
                "Check for updates",
                if self.update_checking {
                    "Checking..."
                } else {
                    "Check"
                },
                cx,
                |this, cx| this.check_for_updates(cx),
            ));
        if let Some(version) = &self.update_available {
            card = card
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.success)
                        .child(format!("Version {version} is available.")),
                )
                .child(
                    self.setting_row_button("Latest release", "Open", cx, |_, _| {
                        let _ = open::that(
                            "https://github.com/ddxtcc/thelounge-native/releases/latest",
                        );
                    }),
                );
        } else if let Some(message) = &self.update_message {
            card = card.child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted)
                    .child(message.clone()),
            );
        }
        card.into_any_element()
    }

    pub(super) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.update_checking {
            return;
        }
        self.update_checking = true;
        self.update_available = None;
        self.update_message = None;
        cx.notify();
        let task = cx.background_spawn(async { crate::update::latest_version() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.update_checking = false;
                match result {
                    Ok(Some(version)) => this.update_available = Some(version),
                    Ok(None) => this.update_message = Some("Lounge Native is up to date.".into()),
                    Err(error) => {
                        this.update_message = Some(format!("Update check failed: {error}"))
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn import_theme(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.toast("Clipboard is empty.");
            return;
        };
        match toml::from_str::<crate::theme::CustomTheme>(&text) {
            Ok(theme) => {
                let saved = crate::theme::SavedTheme::new("Imported", theme);
                self.config.theme = saved.id.clone();
                self.config.saved_themes.push(saved);
                self.apply_theme(cx);
                self.config.save();
                self.toast("Custom theme imported.");
            }
            Err(_) => self.toast("Theme TOML is invalid."),
        }
    }

    pub(super) fn export_theme(&mut self, cx: &mut Context<Self>) {
        let theme = self
            .config
            .saved_themes
            .iter()
            .find(|saved| saved.id == self.config.theme)
            .map(|saved| saved.theme.clone())
            .unwrap_or_else(|| crate::theme::CustomTheme::from_theme(&self.theme));
        match toml::to_string_pretty(&theme) {
            Ok(text) => {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.toast("Theme copied to clipboard.");
            }
            Err(_) => self.toast("Could not export this theme."),
        }
    }

    pub(super) fn theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut row = div().flex().flex_wrap().gap_2();
        let mut themes = crate::theme::THEMES
            .iter()
            .map(|(id, label)| (id.to_string(), label.to_string()))
            .collect::<Vec<_>>();
        themes.extend(
            self.config
                .saved_themes
                .iter()
                .map(|saved| (saved.id.clone(), saved.name.clone())),
        );
        for (id, label) in themes {
            let active = self.config.theme == id;
            let click_id = id.clone();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("theme-{id}").into()))
                    .tab_index(0)
                    .focus(|style| style.bg(theme.hover).text_color(theme.accent))
                    .px_3()
                    .py_2()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if active { theme.accent } else { theme.border })
                    .bg(if active {
                        theme.accent_dim
                    } else {
                        theme.panel
                    })
                    .text_color(if active { theme.accent } else { theme.muted })
                    .text_size(px(12.))
                    .cursor_pointer()
                    .hover(|style| style.border_color(theme.accent))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.config.theme = click_id.clone();
                        this.apply_theme(cx);
                        this.config.save();
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if settings_activation(event) {
                            this.config.theme = id.clone();
                            this.apply_theme(cx);
                            this.config.save();
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }))
                    .child(label.to_string()),
            );
        }
        row.into_any_element()
    }

    pub(super) fn setting_section(&self, title: &str) -> AnyElement {
        div()
            .pt_2()
            .text_size(px(11.))
            .text_color(self.theme.muted)
            .child(title.to_uppercase())
            .into_any_element()
    }

    pub(super) fn setting_row_toggle(
        &self,
        label: &str,
        value: bool,
        cx: &mut Context<Self>,
        toggle: impl Fn(&mut Self) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let toggle = std::rc::Rc::new(toggle);
        let click_toggle = toggle.clone();
        div()
            .id(ElementId::Name(format!("setting-{label}").into()))
            .tab_index(0)
            .focus(|style| style.bg(theme.hover).text_color(theme.accent))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(move |this, _, _, cx| {
                click_toggle(this);
                this.config.save();
                this.apply_theme(cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if settings_activation(event) {
                    toggle(this);
                    this.config.save();
                    this.apply_theme(cx);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .child(label.to_string())
            .child(
                div()
                    .px_2()
                    .py(px(1.))
                    .rounded_full()
                    .text_size(px(11.))
                    .bg(if value { theme.accent } else { theme.hover })
                    .text_color(if value { theme.bg } else { theme.muted })
                    .child(if value { "On" } else { "Off" }),
            )
            .into_any_element()
    }

    pub(super) fn setting_row_cycle(
        &self,
        label: &str,
        value: &str,
        cx: &mut Context<Self>,
        cycle: impl Fn(&mut Self) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let cycle = std::rc::Rc::new(cycle);
        let click_cycle = cycle.clone();
        div()
            .id(ElementId::Name(format!("setting-{label}").into()))
            .tab_index(0)
            .focus(|style| style.bg(theme.hover).text_color(theme.accent))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(move |this, _, _, cx| {
                click_cycle(this);
                this.config.save();
                this.apply_theme(cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if settings_activation(event) {
                    cycle(this);
                    this.config.save();
                    this.apply_theme(cx);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .child(label.to_string())
            .child(
                div()
                    .text_color(theme.muted)
                    .text_size(px(12.))
                    .child(value.to_string()),
            )
            .into_any_element()
    }

    pub(super) fn setting_row_value_button(
        &self,
        label: &str,
        value: &str,
        button: &str,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let action = std::rc::Rc::new(action);
        let click_action = action.clone();
        div()
            .id(ElementId::Name(format!("value-{label}").into()))
            .tab_index(0)
            .focus(|style| style.bg(theme.hover).text_color(theme.accent))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .on_key_down(cx.listener(move |this, event, window, cx| {
                if settings_activation(event) {
                    action(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label.to_string())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .max_w(px(210.))
                            .truncate()
                            .text_color(theme.muted)
                            .text_size(px(12.))
                            .child(value.to_string()),
                    )
                    .child(
                        div()
                            .id(ElementId::Name(format!("choose-{label}").into()))
                            .px_2()
                            .py(px(2.))
                            .rounded(px(4.))
                            .bg(theme.hover)
                            .text_size(px(12.))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.border))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                click_action(this, window, cx)
                            }))
                            .child(button.to_string()),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn setting_row_button(
        &self,
        label: &str,
        button: &str,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let action = std::rc::Rc::new(action);
        let click_action = action.clone();
        div()
            .id(ElementId::Name(format!("action-{label}").into()))
            .tab_index(0)
            .focus(|style| style.bg(theme.hover).text_color(theme.accent))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if settings_activation(event) {
                    action(this, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label.to_string())
            .child(
                div()
                    .id(ElementId::Name(format!("btn-{label}").into()))
                    .px_2()
                    .py(px(2.))
                    .rounded(px(4.))
                    .bg(theme.hover)
                    .text_size(px(12.))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.border))
                    .on_click(cx.listener(move |this, _, _, cx| click_action(this, cx)))
                    .child(button.to_string()),
            )
            .into_any_element()
    }

    pub(super) fn setting_row_action(
        &self,
        label: &str,
        value: &str,
        cx: &mut Context<Self>,
        left: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        right: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let left = std::rc::Rc::new(left);
        let click_left = left.clone();
        let right = std::rc::Rc::new(right);
        let click_right = right.clone();
        div()
            .id(ElementId::Name(format!("action-{label}").into()))
            .tab_index(0)
            .focus(|style| style.bg(theme.hover).text_color(theme.accent))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                let key = event.keystroke.key.to_lowercase();
                if matches!(key.as_str(), "left" | "h") {
                    left(this, cx);
                    cx.stop_propagation();
                } else if matches!(
                    key.as_str(),
                    "right" | "l" | "enter" | "return" | "space" | " "
                ) {
                    right(this, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label.to_string())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id(ElementId::Name(format!("minus-{label}").into()))
                            .px_2()
                            .rounded(px(4.))
                            .bg(theme.hover)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| click_left(this, cx)))
                            .child("-"),
                    )
                    .child(
                        div()
                            .min_w(px(42.))
                            .text_align(gpui::TextAlign::Center)
                            .text_color(theme.muted)
                            .child(value.to_string()),
                    )
                    .child(
                        div()
                            .id(ElementId::Name(format!("plus-{label}").into()))
                            .px_2()
                            .rounded(px(4.))
                            .bg(theme.hover)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| click_right(this, cx)))
                            .child("+"),
                    ),
            )
            .into_any_element()
    }
}
