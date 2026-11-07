use super::tooltip::Tooltip;
use super::*;

pub(super) struct ModalCard {
    card: gpui::Stateful<gpui::Div>,
    body: gpui::Stateful<gpui::Div>,
}

impl ModalCard {
    pub(super) fn child(mut self, child: impl gpui::IntoElement) -> Self {
        self.body = self.body.child(child);
        self
    }

    pub(super) fn into_any_element(self) -> AnyElement {
        self.card.child(self.body).into_any_element()
    }
}

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
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| this.close_overlays(cx)));

        if self.palette.is_some() {
            overlay = overlay.child(self.render_palette(cx));
        } else if let Some(url) = self.lightbox.clone() {
            overlay = overlay.child(self.render_lightbox(&url, window, cx));
        } else if let Some(target) = self.font_picker {
            overlay = overlay.child(self.render_font_picker(target, window, cx));
        } else if self.message_context.is_some() {
            overlay = overlay.child(self.render_message_context(cx));
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
            overlay = overlay.child(self.render_user_menu(menu, window, cx));
        } else if let Some(menu) = self.channel_menu.clone() {
            overlay = overlay.child(self.render_channel_menu(menu, window, cx));
        } else if let Some(menu) = self.message_menu.clone() {
            overlay = overlay.child(self.render_message_menu(menu, window, cx));
        } else if let Some(uuid) = self.confirm_remove_network.clone() {
            overlay = overlay.child(self.render_remove_network(&uuid, cx));
        }

        overlay.into_any_element()
    }

    pub(super) fn close_button(&self, id: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        div()
            .id(id)
            .size(px(26.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.))
            .text_size(px(17.))
            .line_height(relative(1.))
            .text_color(theme.muted)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover).text_color(theme.text))
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_overlays(cx);
                cx.stop_propagation();
            }))
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new("Close", theme)).into())
            .child("×")
            .into_any_element()
    }

    pub(super) fn render_lightbox(
        &self,
        url: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let viewport = window.viewport_size();
        let max_w = viewport.width * 0.88;
        let max_h = viewport.height * 0.82;
        let open_url = url.to_string();
        div()
            .id("lightbox")
            .max_w(max_w)
            .max_h(max_h)
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .id("lightbox-image")
                    .w(max_w)
                    .h(max_h)
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(
                        gpui::img(url.to_string())
                            .image_cache(&self.image_cache)
                            .w_full()
                            .h_full()
                            .object_fit(gpui::ObjectFit::Contain)
                            .with_fallback(move || {
                                div()
                                    .w_full()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(theme.muted)
                                    .child("Image failed to load")
                                    .into_any_element()
                            })
                            .rounded(px(6.)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted)
                            .child("Click outside to close"),
                    )
                    .child(
                        div()
                            .id("lightbox-open")
                            .text_size(px(11.))
                            .text_color(theme.accent)
                            .cursor_pointer()
                            .hover(|style| style.underline())
                            .on_click(cx.listener(move |_, _, _, _| {
                                let _ = open::that(open_url.clone());
                            }))
                            .child("Open in browser"),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn modal_card(&self, title: &str, cx: &mut Context<Self>) -> ModalCard {
        let theme = self.theme;
        let card = div()
            .id("modal-card")
            .w(px(520.))
            .max_h(px(620.))
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
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.to_string()),
                    )
                    .child(self.close_button("modal-close", cx)),
            );
        let body = div()
            .id("modal-body")
            .flex_1()
            .min_h(px(0.))
            .max_h(px(500.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3();
        ModalCard { card, body }
    }

    pub(super) fn menu_card(&self, title: &str, cx: &mut Context<Self>) -> ModalCard {
        let theme = self.theme;
        let card = div()
            .id("menu-card")
            .w(px(420.))
            .max_h(px(680.))
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
                    .flex_shrink_0()
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
                    .child(self.close_button("menu-close", cx)),
            );
        let body = div()
            .id("menu-body")
            .flex_1()
            .min_h(px(0.))
            .max_h(px(560.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_0p5();
        ModalCard { card, body }
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

    pub(super) fn render_message_context(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(context) = self.message_context.clone() else {
            return div().into_any_element();
        };
        let time = format::format_time(
            context.time,
            self.config.show_seconds,
            self.config.use_12h_clock,
        );
        self.menu_card("Message context", cx)
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child(format!("{} - {time}", context.channel)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.accent)
                    .child(context.nick),
            )
            .child(div().text_size(px(13.)).child(context.text))
            .child(self.menu_button("Close", cx, |this, _, cx| {
                this.message_context = None;
                cx.notify();
            }))
            .into_any_element()
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
            let selection = self
                .search_selection
                .min(results.results.len().saturating_sub(1));
            let mut list = div()
                .id("search-results")
                .flex()
                .flex_col()
                .gap_2()
                .max_h(px(420.))
                .overflow_y_scroll()
                .track_scroll(&self.search_scroll);
            for (index, result) in results.results.iter().enumerate() {
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
                let selected = index == selection;
                list = list.child(
                    div()
                        .id(ElementId::Name(format!("search-{}", result.id).into()))
                        .flex()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .cursor_pointer()
                        .when(selected, |this| this.bg(theme.hover))
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.activate_search_result(index, cx);
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
        if !self.fonts_loaded && !self.fonts_loading {
            self.fonts_loading = true;
            let text_system = cx.text_system().clone();
            let task = cx.spawn(async move |this, cx| {
                let fonts = cx
                    .background_executor()
                    .spawn(async move {
                        let mut fonts = text_system.all_font_names();
                        fonts.retain(|font| !font.is_empty() && !font.contains(','));
                        fonts.sort_by_key(|font| font.to_lowercase());
                        fonts.dedup();
                        fonts
                    })
                    .await;
                let _ = this.update(cx, |this, cx| {
                    this.fonts = fonts;
                    this.fonts_loading = false;
                    this.fonts_loaded = true;
                    cx.notify();
                });
            });
            task.detach();
            let spinner = cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(120))
                        .await;
                    let loading = this
                        .update(cx, |this, cx| {
                            if !this.fonts_loading {
                                return false;
                            }
                            this.font_spinner = (this.font_spinner + 1) % 4;
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);
                    if !loading {
                        break;
                    }
                }
            });
            spinner.detach();
        }
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
                    this.child(div().text_color(theme.accent).child("✓"))
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
                        this.child(div().text_color(theme.accent).child("✓"))
                    }),
            );
        }

        let mut card = self
            .modal_card(title, cx)
            .child(self.inputs.font_search.clone())
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child("Type to filter installed fonts."),
            );
        if !self.fonts_loaded {
            card = card.child(
                div()
                    .py_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(theme.accent)
                            .child(["◐", "◓", "◑", "◒"][self.font_spinner % 4]),
                    )
                    .child(div().text_color(theme.muted).child("Loading fonts...")),
            );
        } else if self.fonts.is_empty() {
            card = card.child(
                div()
                    .py_4()
                    .text_color(theme.muted)
                    .child("No fonts found."),
            );
        } else {
            card = card.child(
                div()
                    .id("font-scroll")
                    .max_h(px(420.))
                    .overflow_y_scroll()
                    .child(list),
            );
        }
        card.into_any_element()
    }

    pub(super) fn render_emoji_picker(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let term = self.inputs.emoji_search.read(cx).content().to_string();
        let emoji = crate::emoji::filter(&term, 240);
        let selection = self.emoji_selection.min(emoji.len().saturating_sub(1));
        let mut grid = div()
            .id("emoji-grid")
            .w_full()
            .grid()
            .grid_cols(7)
            .gap_2()
            .max_h(px(420.))
            .overflow_y_scroll()
            .track_scroll(&self.emoji_scroll);
        for (index, item) in emoji.into_iter().enumerate() {
            let glyph = item.glyph;
            let shortcode = item.shortcode;
            grid = grid.child(
                div()
                    .id(ElementId::Name(format!("emoji-{shortcode}").into()))
                    .w_full()
                    .h(px(58.))
                    .px_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .when(index == selection, |this| this.bg(theme.accent_dim))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(cx.listener(move |this, _, _, cx| this.insert_emoji(glyph, cx)))
                    .child(div().text_size(px(22.)).child(glyph))
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .text_center()
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
                    .child("Type to filter; arrows navigate, Enter inserts."),
            )
            .child(grid)
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
            let selection = self
                .mentions_selection
                .min(self.state.mentions.len().saturating_sub(1));
            let mut list = div()
                .id("mention-list")
                .flex()
                .flex_col()
                .gap_2()
                .max_h(px(420.))
                .overflow_y_scroll()
                .track_scroll(&self.mentions_scroll);
            for (index, mention) in self.state.mentions.iter().enumerate() {
                let msg_id = mention.msg_id;
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
                        .when(index == selection, |this| this.bg(theme.accent_dim))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.activate_mention(index, window, cx);
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
                                            cx.stop_propagation();
                                            if let Some(conn) = &this.conn {
                                                conn.emit(
                                                    "mentions:dismiss",
                                                    Some(serde_json::json!(msg_id)),
                                                );
                                            }
                                            this.state.mentions.retain(|m| m.msg_id != msg_id);
                                            cx.notify();
                                        }))
                                        .child("×"),
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
        let text = if label == "Cancel" {
            format!("← {label}")
        } else {
            label.to_string()
        };
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
            .child(text)
            .into_any_element()
    }
}
