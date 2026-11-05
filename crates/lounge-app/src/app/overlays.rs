use super::*;

impl LoungeApp {
    pub(super) fn render_modals(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let mut overlay = div()
            .id("modal-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.overlay)
            .on_click(cx.listener(|this, _, _, cx| this.close_overlays(cx)));

        if let Some(target) = self.font_picker {
            overlay = overlay.child(self.render_font_picker(target, window, cx));
        } else if self.settings_open {
            overlay = overlay.child(self.render_settings(window, cx));
        } else if self.network_modal_open {
            overlay = overlay.child(self.render_network_modal(window, cx));
        } else if self.search_open {
            overlay = overlay.child(self.render_search(window, cx));
        } else if self.mentions_open {
            overlay = overlay.child(self.render_mentions(window, cx));
        } else if self.emoji_picker_open {
            overlay = overlay.child(self.render_emoji_picker(window, cx));
        } else if self.security_open {
            overlay = overlay.child(self.render_security(window, cx));
        } else if let Some(uuid) = self.network_menu.clone() {
            overlay = overlay.child(self.render_network_menu(&uuid, cx));
        } else if let Some(menu) = self.user_menu.clone() {
            overlay = overlay.child(self.render_user_menu(menu, cx));
        } else if let Some(menu) = self.channel_menu.clone() {
            overlay = overlay.child(self.render_channel_menu(menu, cx));
        } else if let Some(uuid) = self.confirm_remove_network.clone() {
            overlay = overlay.child(self.render_remove_network(&uuid, cx));
        }

        overlay.into_any_element()
    }

    pub(super) fn modal_card(
        &self,
        title: &str,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        div()
            .id("modal-card")
            .w(px(520.))
            .max_h(px(620.))
            .overflow_y_scroll()
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .flex()
            .flex_col()
            .gap_3()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .id("modal-close")
                            .px_2()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .text_color(theme.muted)
                            .hover(|style| style.text_color(theme.text))
                            .on_click(cx.listener(|this, _, _, cx| this.close_overlays(cx)))
                            .child("x"),
                    ),
            )
    }

    pub(super) fn menu_card(
        &self,
        title: &str,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        div()
            .id("menu-card")
            .w(px(420.))
            .max_h(px(680.))
            .overflow_y_scroll()
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .rounded(px(10.))
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap_0p5()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .pb_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .id("menu-close")
                            .px_2()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .text_color(theme.muted)
                            .hover(|style| style.text_color(theme.text))
                            .on_click(cx.listener(|this, _, _, cx| this.close_overlays(cx)))
                            .child("x"),
                    ),
            )
    }

    pub(super) fn render_network_modal(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let editing = self.network_edit_uuid.is_some();
        let title = if editing {
            "Edit IRC network"
        } else {
            "Add IRC network"
        };
        let mut card = self
            .modal_card(title, cx)
            .child(self.labeled_input("Name", &self.inputs.network_name));

        let locked = self
            .state
            .configuration
            .as_ref()
            .map(|config| config.lock_network)
            .unwrap_or(false);
        if !locked {
            card = card
                .child(self.labeled_input("Server", &self.inputs.network_host))
                .child(self.labeled_input("Port", &self.inputs.network_port))
                .child(
                    self.setting_row_toggle("Use TLS", self.network_tls, cx, |this| {
                        this.network_tls = !this.network_tls
                    }),
                );
        }
        card = card
            .child(self.labeled_input("Nickname", &self.inputs.network_nick))
            .child(self.labeled_input("Username", &self.inputs.network_username))
            .child(self.labeled_input("Real name", &self.inputs.network_realname))
            .child(self.labeled_input("Server password (optional)", &self.inputs.network_password))
            .child(
                div()
                    .id("create-network")
                    .mt_2()
                    .h(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(theme.accent)
                    .text_color(theme.bg)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.submit_network(cx)))
                    .child(if editing { "Save" } else { "Connect" }),
            );
        card.into_any_element()
    }

    pub(super) fn render_search(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let mut card = self
            .modal_card("Search messages", cx)
            .child(self.inputs.search.clone());

        if let Some(results) = &self.state.search_results {
            if results.results.is_empty() {
                card = card.child(
                    div()
                        .text_color(theme.muted)
                        .child("No results. Search requires message logging on the server."),
                );
            }
            let mut list = div().flex().flex_col().gap_2();
            for result in &results.results {
                let time = format::format_time(
                    result.time,
                    self.config.show_seconds,
                    self.config.use_12h_clock,
                );
                let nick = result
                    .from
                    .as_ref()
                    .map(|from| from.nick.clone())
                    .unwrap_or_default();
                let text = format::clean(&result.text);
                let network_uuid = results.query.network_uuid.clone();
                let channel_name = results.query.channel_name.clone();
                let target = self.state.networks.iter().find_map(|network| {
                    if network.uuid != network_uuid {
                        return None;
                    }
                    network
                        .channels
                        .iter()
                        .find(|channel| channel.chan.name == channel_name)
                        .map(|channel| channel.chan.id)
                });
                list = list.child(
                    div()
                        .id(ElementId::Name(format!("search-{}", result.id).into()))
                        .flex()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(id) = target {
                                this.switch_channel(id, window, cx);
                                this.search_open = false;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(48.))
                                .text_size(px(11.))
                                .text_color(theme.faint)
                                .child(time),
                        )
                        .child(
                            div()
                                .flex_col()
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.accent)
                                        .child(nick),
                                )
                                .child(div().text_size(px(12.)).child(text)),
                        ),
                );
            }
            card = card.child(list);
        }

        card.into_any_element()
    }

    pub(super) fn render_font_picker(
        &mut self,
        target: FontTarget,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let title = match target {
            FontTarget::Ui => "Message font",
            FontTarget::Mono => "Monospace font",
        };
        let current = match target {
            FontTarget::Ui => self.config.font_family.clone(),
            FontTarget::Mono => self.config.monospace_font.clone(),
        };
        let reset_label = match target {
            FontTarget::Ui => "System UI font",
            FontTarget::Mono => "Default monospace",
        };
        let reset_value = match target {
            FontTarget::Ui => ".SystemUIFont",
            FontTarget::Mono => "monospace",
        };
        let term = self.inputs.font_search.read(cx).content().to_lowercase();
        let filtered: Vec<String> = self
            .fonts
            .iter()
            .filter(|font| term.is_empty() || font.to_lowercase().contains(&term))
            .take(250)
            .cloned()
            .collect();

        let mut list = div().id("font-list").flex().flex_col().gap_0p5();
        list = list.child(
            div()
                .id("font-reset")
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .py_1()
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.choose_font(target, reset_value, cx)),
                )
                .child(reset_label)
                .when(current == reset_value, |this| {
                    this.child(div().text_color(theme.accent).child("*"))
                }),
        );
        for font in filtered {
            let selected = font == current;
            let choose = font.clone();
            let preview_font = font.clone();
            list = list.child(
                div()
                    .id(ElementId::Name(format!("font-{font}").into()))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.choose_font(target, &choose, cx)),
                    )
                    .child(div().font_family(preview_font).child(font))
                    .when(selected, |this| {
                        this.child(div().text_color(theme.accent).child("*"))
                    }),
            );
        }

        self.modal_card(title, cx)
            .child(self.inputs.font_search.clone())
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Type to filter installed fonts."),
            )
            .child(
                div()
                    .id("font-scroll")
                    .max_h(px(520.))
                    .overflow_y_scroll()
                    .child(list),
            )
            .into_any_element()
    }

    pub(super) fn render_emoji_picker(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let term = self.inputs.emoji_search.read(cx).content().to_string();
        let emoji = crate::emoji::filter(&term, 240);
        let mut grid = div().id("emoji-grid").flex().flex_wrap().gap_1();
        for item in emoji {
            let glyph = item.glyph;
            let shortcode = item.shortcode;
            grid = grid.child(
                div()
                    .id(ElementId::Name(format!("emoji-{shortcode}").into()))
                    .w(px(58.))
                    .h(px(54.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(cx.listener(move |this, _, _, cx| this.insert_emoji(glyph, cx)))
                    .child(div().text_size(px(22.)).child(glyph))
                    .child(
                        div()
                            .max_w(px(54.))
                            .truncate()
                            .text_size(px(8.))
                            .text_color(theme.muted)
                            .child(shortcode.to_string()),
                    ),
            );
        }

        self.modal_card("Emoji", cx)
            .child(self.inputs.emoji_search.clone())
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Type to filter; click an emoji to insert it."),
            )
            .child(
                div()
                    .id("emoji-scroll")
                    .max_h(px(420.))
                    .overflow_y_scroll()
                    .child(grid),
            )
            .into_any_element()
    }

    pub(super) fn render_mentions(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let mut card = self.modal_card("Recent mentions", cx);
        if self.state.mentions.is_empty() {
            card = card.child(
                div()
                    .text_color(theme.muted)
                    .child("You have no recent mentions."),
            );
        } else {
            let mut list = div().flex().flex_col().gap_2();
            for mention in &self.state.mentions {
                let msg_id = mention.msg_id;
                let chan_id = mention.chan_id;
                let nick = mention.from.nick.clone();
                let text = format::clean(&mention.text);
                let time = format::format_time(
                    mention.time,
                    self.config.show_seconds,
                    self.config.use_12h_clock,
                );
                list = list.child(
                    div()
                        .id(ElementId::Name(format!("mention-{msg_id}").into()))
                        .px_2()
                        .py_2()
                        .rounded(px(5.))
                        .bg(theme.highlight_bg)
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.state.find_channel(chan_id).is_some() {
                                this.switch_channel(chan_id, window, cx);
                                this.mentions_open = false;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .text_size(px(11.))
                                .text_color(theme.muted)
                                .child(format!("{nick} - {time}"))
                                .child(
                                    div()
                                        .id(ElementId::Name(format!("dismiss-{msg_id}").into()))
                                        .px_1()
                                        .cursor_pointer()
                                        .hover(|style| style.text_color(theme.error))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(conn) = &this.conn {
                                                conn.emit(
                                                    "mentions:dismiss",
                                                    Some(serde_json::json!(msg_id)),
                                                );
                                            }
                                            this.state.mentions.retain(|m| m.msg_id != msg_id);
                                            cx.notify();
                                        }))
                                        .child("x"),
                                ),
                        )
                        .child(div().text_size(px(13.)).child(text)),
                );
            }
            card = card.child(list);
        }
        card.into_any_element()
    }
    pub(super) fn menu_button(
        &self,
        label: &str,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let danger = label.starts_with("Remove") || label.starts_with("Leave");
        div()
            .id(ElementId::Name(format!("menu-{label}").into()))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py(px(7.))
            .rounded(px(5.))
            .cursor_pointer()
            .text_color(if danger { theme.error } else { theme.text })
            .hover(|style| {
                style.bg(if danger {
                    theme.error.opacity(0.12)
                } else {
                    theme.hover
                })
            })
            .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
            .child(label.to_string())
            .into_any_element()
    }
}
