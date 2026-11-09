use super::*;

impl LoungeApp {
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
                    .tab_index(0)
                    .focus(|style| style.bg(theme.border).text_color(theme.text))
                    .px_3()
                    .py(px(5.))
                    .rounded(px(5.))
                    .bg(theme.accent)
                    .text_color(theme.bg)
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.change_password(cx)))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if activation_key(event) {
                            this.change_password(cx);
                            cx.stop_propagation();
                        }
                    }))
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
                            .tab_index(0)
                            .focus(|style| style.bg(theme.border).text_color(theme.text))
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
                            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                                if activation_key(event) {
                                    if let Some(conn) = &this.conn {
                                        conn.emit("sessions:get", None);
                                    }
                                    cx.notify();
                                    cx.stop_propagation();
                                }
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
                            .tab_index(0)
                            .focus(|style| style.bg(theme.border).text_color(theme.text))
                            .px_2()
                            .py(px(2.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(11.))
                            .text_color(theme.muted)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.error))
                            .on_click(cx.listener({
                                let token = token.clone();
                                move |this, _, _, _| {
                                    if let Some(conn) = &this.conn {
                                        conn.emit("sign-out", Some(serde_json::json!(token)));
                                    }
                                }
                            }))
                            .on_key_down(cx.listener({
                                let token = token.clone();
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activation_key(event) {
                                        if let Some(conn) = &this.conn {
                                            conn.emit("sign-out", Some(serde_json::json!(token)));
                                        }
                                        cx.stop_propagation();
                                    }
                                }
                            }))
                            .child("Sign out"),
                    ),
            );
        }
        section.into_any_element()
    }
}
