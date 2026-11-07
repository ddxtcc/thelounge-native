use super::message_text::{build_message_text, is_status_message, user_mode_prefix_in_message};
use super::*;

impl LoungeApp {
    pub(super) fn render_messages(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(active) = self.state.active else {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted)
                .child("No channel selected")
                .into_any_element();
        };
        let scroll_offset = self.messages_scroll.offset();
        let scroll_max = self.messages_scroll.max_offset();
        let was_at_bottom = self.last_scroll_max.height <= px(1.)
            || scroll_offset.y <= -self.last_scroll_max.height + px(48.);
        if scroll_max.height > self.last_scroll_max.height + px(1.) && was_at_bottom {
            self.schedule_scroll_bottom(cx);
        } else if let Some(preview_message) = self.preview_scroll_message.take()
            && scroll_max.height > self.last_scroll_max.height + px(1.)
            && !was_at_bottom
            && self
                .state
                .active_channel()
                .and_then(|(_, channel)| channel.chan.messages.last())
                .map(|message| message.id)
                != Some(preview_message)
        {
            let delta = scroll_max.height - self.last_scroll_max.height;
            let offset = self.messages_scroll.offset();
            let y = (offset.y - delta).min(px(0.)).max(-scroll_max.height);
            self.messages_scroll.set_offset(Point { x: offset.x, y });
        }
        self.preview_scroll_message = None;
        self.last_scroll_max = scroll_max;
        let Some((network, channel)) = self.state.find_channel(active) else {
            return div().flex_1().into_any_element();
        };
        if let Some(special) = channel.chan.special.clone() {
            return self.render_special(network, channel, &special, cx);
        }

        let rows = build_rows(channel, &self.config);
        for row in &rows {
            match row {
                Row::Message(message) => {
                    self.message_anchors.entry(message.id).or_insert_with(|| {
                        gpui::ScrollAnchor::for_handle(self.messages_scroll.clone())
                    });
                }
                Row::Condensed { messages, .. } => {
                    for message in messages {
                        self.message_anchors.entry(message.id).or_insert_with(|| {
                            gpui::ScrollAnchor::for_handle(self.messages_scroll.clone())
                        });
                    }
                }
                _ => {}
            }
        }
        let nick_width = if self.config.nick_column && !self.narrow {
            nick_column_width(channel, self.config.font_size)
        } else {
            0.
        };
        let status_column = if nick_width > 0. { nick_width } else { 22. };
        let summary_indent =
            px(16. + if self.config.show_timestamps { 62. } else { 0. } + status_column + 8.);
        let mut content = div().flex().flex_col().w_full().py_2();

        if channel.more_history_available() {
            let label = if channel.history_loading {
                "Loading older messages...".to_string()
            } else {
                "Show older messages".to_string()
            };
            content = content.child(
                div().w_full().flex().justify_center().py_2().child(
                    div()
                        .id("load-more")
                        .px_3()
                        .py_1()
                        .rounded(px(5.))
                        .bg(theme.panel)
                        .border_1()
                        .border_color(theme.border)
                        .text_size(px(12.))
                        .text_color(theme.muted)
                        .flex_shrink_0()
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(id) = this.state.active
                                && let Some(last_id) = this.state.mark_history_requested(id)
                                && let Some(conn) = &this.conn
                            {
                                conn.more(
                                    id,
                                    last_id,
                                    this.config.status_messages != StatusMessages::Shown,
                                );
                            }
                            cx.notify();
                        }))
                        .child(label),
                ),
            );
        }

        for row in rows {
            match row {
                Row::Date(label) => {
                    content = content.child(
                        div()
                            .px_4()
                            .pt_3()
                            .pb_1()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(div().flex_1().h(px(1.)).bg(theme.success))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.success)
                                    .child(label),
                            )
                            .child(div().flex_1().h(px(1.)).bg(theme.success)),
                    );
                }
                Row::Unread => {
                    content = content.child(
                        div()
                            .px_4()
                            .py_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().h(px(1.)).bg(theme.accent))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme.accent)
                                    .child("NEW"),
                            )
                            .child(div().flex_1().h(px(1.)).bg(theme.accent)),
                    );
                }
                Row::Message(message) => {
                    content = content
                        .child(self.render_message_row(network, channel, message, nick_width, cx));
                }
                Row::Condensed { id, messages } => {
                    let summary = condensed_summary(&messages);
                    let expanded = self.expanded_condensed.contains(&id);
                    content = content.child(
                        div()
                            .id(ElementId::Name(format!("cond-{id}").into()))
                            .pl(summary_indent)
                            .pr_4()
                            .py(px(1.))
                            .cursor_pointer()
                            .text_size(px(12.))
                            .text_color(theme.muted)
                            .hover(|style| style.bg(theme.hover))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.expanded_condensed.remove(&id) {
                                    this.expanded_condensed.insert(id);
                                }
                                cx.notify();
                            }))
                            .child(format!("{summary} {}", if expanded { "v" } else { "»" })),
                    );
                    if expanded {
                        for message in messages {
                            content = content.child(
                                self.render_message_row(network, channel, message, nick_width, cx),
                            );
                        }
                    }
                }
            }
        }

        if let Some(message_id) = self.pending_focus_message.take() {
            if let Some(anchor) = self.message_anchors.get(&message_id) {
                anchor.scroll_to(window, cx);
            } else {
                self.toast("Message is not loaded.");
            }
        }
        div()
            .id("messages")
            .flex_1()
            .w_full()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.messages_scroll)
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if !this.selecting_text {
                    return;
                }
                let bounds = this.messages_scroll.bounds();
                let edge = px(32.);
                let delta = if event.position.y < bounds.top() + edge {
                    px(18.)
                } else if event.position.y > bounds.bottom() - edge {
                    px(-18.)
                } else {
                    px(0.)
                };
                if delta != px(0.) {
                    this.scroll_messages_by(delta, cx);
                }
            }))
            .child(content)
            .into_any_element()
    }

    pub(super) fn render_message_row(
        &self,
        network: &NetworkState,
        channel: &ChannelState,
        message: &Message,
        nick_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let highlight = message.highlight;
        let is_self = message.is_self;
        let time = format::format_time(
            message.time,
            self.config.show_seconds,
            self.config.use_12h_clock,
        );
        let nick_column = self.config.nick_column && !self.narrow;
        let body = build_message_text(
            message,
            network,
            channel,
            &self.config,
            nick_column,
            &theme,
            self.theme.dark,
        );
        let (text, mut runs, ranges, targets) = body;
        if let Some(range) = self
            .text_selection
            .as_ref()
            .and_then(|selection| selection.range_for(channel, message.id, text.len()))
        {
            runs = select_runs(runs, range, theme.accent_dim);
        }
        let selection_text = text.clone();

        let weak = cx.entity().downgrade();
        let targets = Rc::new(targets);
        let click = {
            let weak = weak.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                let Some(target) = targets.get(index).cloned() else {
                    return;
                };
                match target {
                    ClickTarget::Url(url) => {
                        let _ = open::that(url);
                    }
                    ClickTarget::Nick(nick) => {
                        let _ = weak.update(cx, |this, cx| this.insert_nick(&nick, cx));
                    }
                }
            }
        };

        let message_id = message.id;
        let hover = {
            let weak = weak.clone();
            move |index: Option<usize>,
                  event: gpui::MouseMoveEvent,
                  _: &mut Window,
                  cx: &mut App| {
                let Some(index) = index else {
                    return;
                };
                let _ = weak.update(cx, |this, cx| {
                    this.hovered_text = Some((message_id, index));
                    if event.dragging()
                        && this.selecting_text
                        && let Some(selection) = &mut this.text_selection
                    {
                        selection.end = SelectionPoint { message_id, index };
                        cx.notify();
                    }
                });
            }
        };
        let mut text_element = InteractiveText::new(
            ElementId::Name(format!("msg-{message_id}").into()),
            StyledText::new(text.clone()).with_runs(runs),
        )
        .on_hover(hover);
        if !ranges.is_empty() {
            text_element = text_element.on_click(ranges, click);
        }
        let text_element = div()
            .id(ElementId::Name(format!("msg-wrap-{message_id}").into()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    window.focus(&this.root_focus);
                    let index = this
                        .hovered_text
                        .filter(|(id, _)| *id == message_id)
                        .map(|(_, index)| index)
                        .unwrap_or(0);
                    let point = SelectionPoint { message_id, index };
                    if event.click_count >= 2 {
                        let range = word_range(&selection_text, index);
                        this.text_selection = Some(TextSelection {
                            start: SelectionPoint {
                                message_id,
                                index: range.start,
                            },
                            end: SelectionPoint {
                                message_id,
                                index: range.end,
                            },
                        });
                        this.selecting_text = false;
                    } else if event.modifiers.shift
                        && let Some(selection) = &mut this.text_selection
                    {
                        selection.end = point;
                        this.selecting_text = false;
                    } else {
                        this.text_selection = Some(TextSelection {
                            start: point,
                            end: point,
                        });
                        this.selecting_text = true;
                    }
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.selecting_text = false;
                    if this
                        .text_selection
                        .as_ref()
                        .is_some_and(TextSelection::is_empty)
                    {
                        this.text_selection = None;
                    }
                    cx.notify();
                }),
            )
            .child(text_element)
            .into_any_element();

        let reply_button = if message.msgid.is_some() && message.kind != MessageType::Unknown {
            let message = message.clone();
            Some(
                div()
                    .id(ElementId::Name(format!("reply-{}", message.id).into()))
                    .px_1()
                    .rounded(px(3.))
                    .text_size(px(11.))
                    .text_color(theme.faint)
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.accent))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_reply(&message, cx)))
                    .child(">"),
            )
        } else {
            None
        };

        let mut previews = div().flex().flex_col().gap_1();
        let mut inline_images: Vec<String> = Vec::new();
        for preview in &message.previews {
            if !self.config.show_inline_images {
                break;
            }
            if preview.kind == "image"
                || preview
                    .media_type
                    .as_deref()
                    .map(|kind| kind.starts_with("image/"))
                    .unwrap_or(false)
            {
                let candidate = if preview.kind == "image" {
                    preview
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
                        .unwrap_or(&preview.link)
                } else {
                    preview
                        .thumb_actual_url
                        .as_deref()
                        .filter(|url| !url.is_empty())
                        .or_else(|| Some(preview.thumb.as_str()).filter(|url| !url.is_empty()))
                        .or_else(|| preview.media.as_deref().filter(|url| !url.is_empty()))
                        .unwrap_or(&preview.link)
                };
                inline_images.push(self.absolute_media_url(candidate));
            }
        }
        for link in format::find_links(&message.text) {
            if !self.config.show_inline_images {
                break;
            }
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
                inline_images.push(self.absolute_media_url(&link.url));
            }
        }
        inline_images.sort();
        inline_images.dedup();

        for preview in &message.previews {
            if !self.config.show_previews
                || preview.head.is_empty()
                || preview.shown == Some(false)
                || preview.kind == "image"
            {
                continue;
            }
            let url = preview.link.clone();
            let label = if preview.body.is_empty() {
                preview.head.clone()
            } else {
                format!("{} - {}", preview.head, preview.body)
            };
            let thumb = preview
                .thumb_actual_url
                .as_deref()
                .filter(|url| !url.is_empty())
                .or_else(|| Some(preview.thumb.as_str()).filter(|url| !url.is_empty()))
                .or_else(|| {
                    (preview.kind == "image")
                        .then_some(preview.media.as_deref())
                        .flatten()
                        .filter(|url| !url.is_empty())
                })
                .map(|url| self.absolute_media_url(url));
            let media_type = preview.media_type.as_deref().unwrap_or_default();
            let video = preview.kind == "video" || media_type.starts_with("video/");
            let audio = preview.kind == "audio" || media_type.starts_with("audio/");
            let playable = video || audio;
            let play_url = self.absolute_media_url(
                preview
                    .media
                    .as_deref()
                    .filter(|url| !url.is_empty())
                    .unwrap_or(&preview.link),
            );
            let mut card = div()
                .id(ElementId::Name(
                    format!("preview-{}-{}", message.id, preview.link).into(),
                ))
                .px_2()
                .py_1()
                .rounded(px(4.))
                .bg(theme.panel)
                .border_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .text_color(theme.text)
                .cursor_pointer()
                .hover(|style| style.border_color(theme.accent))
                .flex()
                .flex_col()
                .gap_1()
                .child(div().truncate().child(label));
            if playable {
                card = card
                    .on_click(cx.listener(move |_, _, _, _| {
                        let _ = open::that(play_url.clone());
                    }))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.accent)
                            .child(if video { "Play video" } else { "Play audio" }),
                    );
                if let Some(thumb) = thumb {
                    card = card.child(
                        div()
                            .w(px(220.))
                            .h(px(140.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                gpui::img(thumb)
                                    .image_cache(&self.image_cache)
                                    .w(px(220.))
                                    .h(px(140.))
                                    .object_fit(gpui::ObjectFit::Contain)
                                    .with_fallback(move || image_fallback(theme, "Image failed"))
                                    .rounded(px(4.)),
                            ),
                    );
                }
            } else if let Some(thumb) = thumb {
                let lightbox_url = thumb.clone();
                card = card
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.lightbox = Some(lightbox_url.clone());
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(220.))
                            .h(px(140.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                gpui::img(thumb)
                                    .image_cache(&self.image_cache)
                                    .w(px(220.))
                                    .h(px(140.))
                                    .object_fit(gpui::ObjectFit::Contain)
                                    .with_fallback(move || image_fallback(theme, "Image failed"))
                                    .rounded(px(4.)),
                            ),
                    );
            } else {
                card = card.on_click(cx.listener(move |_, _, _, _| {
                    let _ = open::that(url.clone());
                }));
            }
            previews = previews.child(card);
        }

        let mut inline = div().flex().flex_col().gap_2();
        let image_height = if self.narrow { px(170.) } else { px(240.) };
        for (index, url) in inline_images.iter().enumerate() {
            let lightbox_url = url.clone();
            inline = inline.child(
                div()
                    .id(ElementId::Name(
                        format!("inline-image-{}-{index}", message.id).into(),
                    ))
                    .w_full()
                    .max_w(px(460.))
                    .h(image_height)
                    .rounded(px(6.))
                    .bg(theme.panel)
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.lightbox = Some(lightbox_url.clone());
                        cx.notify();
                    }))
                    .child(
                        gpui::img(url.clone())
                            .image_cache(&self.image_cache)
                            .w_full()
                            .h(image_height)
                            .max_h(image_height)
                            .object_fit(gpui::ObjectFit::Contain)
                            .with_fallback(move || image_fallback(theme, "Image failed to load"))
                            .rounded(px(6.)),
                    ),
            );
        }

        let mut content = div()
            .flex_1()
            .min_w(px(0.))
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(text_element)
                    .child(inline)
                    .child(previews),
            );

        if let Some(reply_button) = reply_button {
            content = content.child(reply_button);
        }

        let context_message = message.clone();
        let focused = self.focused_message == Some(message.id);
        let anchor = self.message_anchors.get(&message.id).cloned();
        let mut row = div()
            .id(ElementId::Name(format!("row-{}", message.id).into()))
            .flex()
            .w_full()
            .items_start()
            .gap_2()
            .px_4()
            .py(px(self.config.message_spacing))
            .when(highlight || focused, |this| {
                this.bg(if focused {
                    theme.accent_dim
                } else {
                    theme.highlight_bg
                })
            })
            .when(is_self, |this| this.opacity(0.85))
            .anchor_scroll(anchor)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.message_menu = Some(MessageMenu {
                        position: event.position,
                        message: context_message.clone(),
                    });
                    cx.notify();
                }),
            );

        if self.config.show_timestamps {
            row = row.child(
                div()
                    .w(px(54.))
                    .flex_shrink_0()
                    .text_size(px(11.))
                    .text_color(theme.faint)
                    .child(time),
            );
        }
        let status = is_status_message(message.kind);
        let status_column = if nick_column { nick_width } else { 22. };
        if status {
            let (icon, color) = status_icon(message.kind, &theme);
            row = row.child(
                div()
                    .w(px(status_column))
                    .flex_shrink_0()
                    .flex()
                    .justify_end()
                    .pr_2()
                    .text_size(px(12.))
                    .text_color(color)
                    .child(icon),
            );
        } else if nick_column {
            let nick = message
                .from
                .as_ref()
                .map(|from| from.nick.clone())
                .unwrap_or_default();
            let mode = message
                .from
                .as_ref()
                .and_then(|from| user_mode_prefix_in_message(from, network))
                .unwrap_or_default();
            let color = if self.config.colored_nicks && !nick.is_empty() {
                theme.nick_color(&nick)
            } else {
                theme.text
            };
            let insert = nick.clone();
            let mut column = div()
                .id(ElementId::Name(format!("nickcol-{}", message.id).into()))
                .w(px(nick_width))
                .flex_shrink_0()
                .pr_2()
                .flex()
                .justify_end()
                .truncate()
                .text_size(px(self.config.font_size))
                .font_weight(FontWeight::BOLD)
                .text_color(color);
            if !nick.is_empty() {
                column = column
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.accent))
                    .on_click(cx.listener(move |this, _, _, cx| this.insert_nick(&insert, cx)))
                    .child(format!("{mode}{nick}"));
            } else {
                column = column.child("");
            }
            row = row.child(column);
        }
        row.child(content).into_any_element()
    }
}

fn build_rows<'a>(channel: &'a ChannelState, config: &AppConfig) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    let mut last_date: Option<chrono::NaiveDate> = None;
    let mut unread_shown = false;
    let show_unread = channel.chan.unread > 0 || channel.chan.highlight > 0;
    let message_count = channel.chan.messages.len();
    let mut index = 0;

    while index < message_count {
        let message = &channel.chan.messages[index];
        let mut condensed_end = index + 1;
        let condensable = channel.is_channel() || channel.is_query();
        if condensable
            && config.status_messages != StatusMessages::Hidden
            && config.status_messages == StatusMessages::Condensed
            && message.kind.is_condensable()
            && !message.is_self
            && !message.highlight
        {
            while condensed_end < message_count {
                let next = &channel.chan.messages[condensed_end];
                if !next.kind.is_condensable() || next.is_self || next.highlight {
                    break;
                }
                condensed_end += 1;
            }
        }

        let first_id = message.id;
        let group = &channel.chan.messages[index..condensed_end];
        if show_unread
            && !unread_shown
            && channel.chan.first_unread > 0
            && first_id >= channel.chan.first_unread
        {
            rows.push(Row::Unread);
            unread_shown = true;
        }

        let time = message.time;
        if let Some(date) = time.map(|time| time.with_timezone(&Local).date_naive())
            && last_date != Some(date)
        {
            rows.push(Row::Date(date_label(date)));
            last_date = Some(date);
        }

        if group.len() > 1 {
            rows.push(Row::Condensed {
                id: first_id,
                messages: group.iter().collect(),
            });
        } else if group.len() == 1
            && !(config.status_messages == StatusMessages::Hidden && group[0].kind.is_condensable())
        {
            rows.push(Row::Message(&group[0]));
        }

        index = condensed_end;
    }

    if config.status_messages == StatusMessages::Hidden {
        rows.retain(|row| !matches!(row, Row::Message(message) if message.kind.is_condensable()));
    }

    rows
}

enum Row<'a> {
    Date(String),
    Unread,
    Message(&'a Message),
    Condensed { id: i64, messages: Vec<&'a Message> },
}

fn nick_column_width(channel: &ChannelState, font_size: f32) -> f32 {
    let mut chars = 0;
    for user in &channel.users {
        let width = user.nick.chars().count() + usize::from(!user.mode.is_empty());
        chars = chars.max(width);
    }
    for message in &channel.chan.messages {
        if let Some(from) = &message.from {
            let width = from.nick.chars().count() + usize::from(!from.mode.is_empty());
            chars = chars.max(width);
        }
    }
    (chars as f32 * font_size * 0.58 + 18.).clamp(70., 180.)
}

fn word_range(text: &str, index: usize) -> Range<usize> {
    let index = index.min(text.len());
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_' || ch == '-';
    let mut start = index;
    while let Some(ch) = text[..start].chars().next_back() {
        if !is_word(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let mut end = index;
    while let Some(ch) = text[end..].chars().next() {
        if !is_word(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    start..end
}

fn image_fallback(theme: Theme, label: &'static str) -> AnyElement {
    div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .text_color(theme.muted)
        .child(label)
        .into_any_element()
}

fn select_runs(runs: Vec<TextRun>, range: Range<usize>, color: Hsla) -> Vec<TextRun> {
    if range.is_empty() {
        return runs;
    }
    let mut result = Vec::with_capacity(runs.len());
    let mut offset = 0;
    for run in runs {
        let start = offset;
        let end = offset + run.len;
        offset = end;
        if end <= range.start || start >= range.end {
            result.push(run);
            continue;
        }
        let selected_start = range.start.max(start);
        let selected_end = range.end.min(end);
        let before = selected_start - start;
        if before > 0 {
            let mut part = run.clone();
            part.len = before;
            result.push(part);
        }
        let inside = selected_end - selected_start;
        if inside > 0 {
            let mut part = run.clone();
            part.len = inside;
            part.background_color = Some(color);
            result.push(part);
        }
        let after = end - selected_end;
        if after > 0 {
            let mut part = run;
            part.len = after;
            result.push(part);
        }
    }
    result
}

fn status_icon(kind: MessageType, theme: &Theme) -> (&'static str, Hsla) {
    match kind {
        MessageType::Join
        | MessageType::Login
        | MessageType::Logout
        | MessageType::Nick
        | MessageType::Chghost
        | MessageType::Invite => ("→", theme.success),
        MessageType::Part | MessageType::Quit | MessageType::Kick => ("←", theme.error),
        MessageType::Topic | MessageType::TopicSetBy => ("!", theme.success),
        MessageType::Mode | MessageType::ModeChannel | MessageType::ModeUser => {
            ("*", theme.success)
        }
        MessageType::Away | MessageType::Back => ("o", theme.muted),
        MessageType::Error | MessageType::Warn => ("!", theme.error),
        MessageType::Notice => ("-", theme.muted),
        MessageType::Ctcp | MessageType::CtcpRequest | MessageType::Raw => (">", theme.muted),
        _ => ("*", theme.muted),
    }
}

fn date_label(date: chrono::NaiveDate) -> String {
    let today = Local::now().date_naive();
    if date == today {
        "Today".into()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".into()
    } else {
        date.format("%e %B %Y").to_string().trim().to_string()
    }
}

fn condensed_summary(messages: &[&Message]) -> String {
    let mut joins = 0;
    let mut parts = 0;
    let mut nicks = 0;
    let mut kicks = 0;
    let mut modes = 0;
    let mut away = 0;
    let mut back = 0;
    let mut chghost = 0;
    for message in messages {
        match message.kind {
            MessageType::Join => joins += 1,
            MessageType::Part | MessageType::Quit => parts += 1,
            MessageType::Nick => nicks += 1,
            MessageType::Kick => kicks += 1,
            MessageType::Mode => {
                modes += message
                    .text
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| *c != '+' && *c != '-')
                    .count()
                    .max(1)
            }
            MessageType::Away => away += 1,
            MessageType::Back => back += 1,
            MessageType::Chghost => chghost += 1,
            _ => {}
        }
    }
    let mut strings = Vec::new();
    let plural = |count: usize, one: &str, many: &str| {
        if count == 1 {
            format!("1 user {one}")
        } else if count > 1 {
            format!("{count} users {many}")
        } else {
            String::new()
        }
    };
    for value in [
        if away == 1 {
            "marked away once".into()
        } else if away > 1 {
            format!("marked away {away} times")
        } else {
            String::new()
        },
        if back == 1 {
            "marked back once".into()
        } else if back > 1 {
            format!("marked back {back} times")
        } else {
            String::new()
        },
        if chghost == 1 {
            "1 user has changed hostname".into()
        } else if chghost > 1 {
            format!("{chghost} users have changed hostname")
        } else {
            String::new()
        },
        plural(joins, "has joined", "have joined"),
        plural(kicks, "was kicked", "were kicked"),
        if modes == 1 {
            "1 mode was set".into()
        } else if modes > 1 {
            format!("{modes} modes were set")
        } else {
            String::new()
        },
        plural(nicks, "has changed nick", "have changed nicks"),
        plural(parts, "has left", "have left"),
    ] {
        if !value.is_empty() {
            strings.push(value);
        }
    }
    if strings.is_empty() {
        format!("{} status messages", messages.len())
    } else if strings.len() == 1 {
        strings[0].clone()
    } else if strings.len() == 2 {
        format!("{} and {}", strings[0], strings[1])
    } else {
        let last = strings.last().cloned().unwrap_or_default();
        format!("{}, and {last}", strings[..strings.len() - 1].join(", "))
    }
}
