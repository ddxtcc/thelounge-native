use super::*;

impl LoungeApp {
    pub(super) fn render_settings(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut card = self.modal_card("Settings", cx);
        card = card.child(self.settings_tabs(cx));
        card = match self.settings_tab {
            SettingsTab::Appearance => card.child(self.settings_appearance(cx)),
            SettingsTab::Messages => card.child(self.settings_messages(cx)),
            SettingsTab::General => card.child(self.settings_general(cx)),
            SettingsTab::Server => card.child(self.settings_server(cx)),
            SettingsTab::About => card.child(self.settings_about()),
        };
        card.into_any_element()
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
            .child(
                self.setting_row_toggle("Icon buttons", self.config.icon_buttons, cx, |this| {
                    this.config.icon_buttons = !this.config.icon_buttons
                }),
            );
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
            .child(self.setting_row_toggle(
                "Show inline images",
                self.config.show_inline_images,
                cx,
                |this| this.config.show_inline_images = !this.config.show_inline_images,
            ))
            .child(self.setting_row_toggle(
                "Show link previews",
                self.config.show_previews,
                cx,
                |this| this.config.show_previews = !this.config.show_previews,
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
                "Desktop notifications",
                self.config.notifications,
                cx,
                |this| this.config.notifications = !this.config.notifications,
            ));
        #[cfg(target_os = "linux")]
        {
            card = card
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
                    if self.config.enable_global_hotkey {
                        "Ctrl+Shift+L"
                    } else {
                        "Disabled"
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

    fn settings_about(&self) -> AnyElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap_3()
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
            .into_any_element()
    }

    pub(super) fn import_theme(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.toast("Clipboard is empty.");
            return;
        };
        match toml::from_str::<crate::theme::CustomTheme>(&text) {
            Ok(theme) => {
                self.config.custom_theme = Some(theme);
                self.config.theme = "custom".into();
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
            .custom_theme
            .clone()
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
        let mut themes = crate::theme::THEMES.to_vec();
        if self.config.custom_theme.is_some() {
            themes.push(("custom", "Custom"));
        }
        for (id, label) in themes {
            let active = self.config.theme == *id;
            let id = id.to_string();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("theme-{id}").into()))
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
                        this.config.theme = id.clone();
                        this.apply_theme(cx);
                        this.config.save();
                        cx.notify();
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
        div()
            .id(ElementId::Name(format!("setting-{label}").into()))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(move |this, _, _, cx| {
                toggle(this);
                this.config.save();
                this.apply_theme(cx);
                cx.notify();
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
        div()
            .id(ElementId::Name(format!("setting-{label}").into()))
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(move |this, _, _, cx| {
                cycle(this);
                this.config.save();
                this.apply_theme(cx);
                cx.notify();
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
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
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
                            .on_click(
                                cx.listener(move |this, _, window, cx| action(this, window, cx)),
                            )
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
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
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
                    .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
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
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
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
                            .on_click(cx.listener(move |this, _, _, cx| left(this, cx)))
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
                            .on_click(cx.listener(move |this, _, _, cx| right(this, cx)))
                            .child("+"),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_security(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut card = self.modal_card("Security & sessions", cx);
        card = card
            .child(self.security_section(cx))
            .child(self.sessions_section(cx));
        card.into_any_element()
    }

    pub(super) fn security_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let configuration = self.state.configuration.clone();
        let enabled = configuration
            .as_ref()
            .map(|config| !config.public && !config.ldap_enabled)
            .unwrap_or(false);
        if !enabled {
            return div()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted)
                        .child("Password changes are managed by the server."),
                )
                .into_any_element();
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.setting_section("Password"))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Press Enter in any field or use the button to change your password."),
            )
            .child(self.labeled_input("Current password", &self.inputs.password_old))
            .child(self.labeled_input("New password", &self.inputs.password_new))
            .child(self.labeled_input("Verify new password", &self.inputs.password_verify))
            .child(
                div()
                    .id("change-password")
                    .px_3()
                    .py(px(5.))
                    .rounded(px(5.))
                    .bg(theme.accent)
                    .text_color(theme.bg)
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.change_password(cx)))
                    .child("Change password"),
            )
            .into_any_element()
    }

    pub(super) fn sessions_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut section = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.setting_section("Sessions"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted)
                            .child(format!("{} active session(s)", self.state.sessions.len())),
                    )
                    .child(
                        div()
                            .id("refresh-sessions")
                            .px_2()
                            .py(px(2.))
                            .rounded(px(4.))
                            .bg(theme.hover)
                            .text_size(px(11.))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(conn) = &this.conn {
                                    conn.emit("sessions:get", None);
                                }
                                cx.notify();
                            }))
                            .child("Refresh"),
                    ),
            );
        for session in &self.state.sessions {
            let token = session.token.clone();
            let current = session.current;
            let label = format!(
                "{} - {} - {}",
                if current { "This session" } else { "Session" },
                session.ip,
                if session.agent.is_empty() {
                    "unknown client".to_string()
                } else {
                    session.agent.chars().take(48).collect()
                }
            );
            section = section.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py(px(5.))
                    .rounded(px(4.))
                    .hover(|style| style.bg(theme.hover))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .text_size(px(11.))
                            .text_color(if current { theme.text } else { theme.muted })
                            .child(label),
                    )
                    .child(
                        div()
                            .id(ElementId::Name(format!("session-{token}").into()))
                            .px_2()
                            .py(px(2.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(11.))
                            .text_color(theme.muted)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.error))
                            .on_click(cx.listener(move |this, _, _, _| {
                                if let Some(conn) = &this.conn {
                                    conn.emit("sign-out", Some(serde_json::json!(token)));
                                }
                            }))
                            .child("Sign out"),
                    ),
            );
        }
        section.into_any_element()
    }
}
