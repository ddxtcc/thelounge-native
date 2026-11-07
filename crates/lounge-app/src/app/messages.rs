use super::message_list::{MessageRowsCache, Row, build_rows, nick_column_width};
use super::message_text::{
    build_message_text, image_fallback, is_status_message, user_mode_prefix_in_message,
};
use super::*;

impl LoungeApp {
    pub(super) fn render_messages(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.image_cache_current.borrow_mut().clear();
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
        let (rows, nick_width, summary_indent, cache_update) = {
            let Some((network, channel)) = self.state.find_channel(active) else {
                return div().flex_1().into_any_element();
            };
            if let Some(special) = channel.chan.special.clone() {
                return self.render_special(network, channel, &special, cx);
            }
            let more = channel.more_history_available();
            let loading = channel.history_loading;
            let key = (
                channel.chan.id,
                self.state.revision,
                self.config.status_messages,
                more,
                loading,
            );
            let cached = self
                .message_rows_cache
                .as_ref()
                .filter(|cache| {
                    cache.channel_id == key.0
                        && cache.revision == key.1
                        && cache.status == key.2
                        && cache.more == key.3
                        && cache.loading == key.4
                })
                .map(|cache| cache.rows.clone());
            let (rows, cache_update) = if let Some(rows) = cached {
                (rows, None)
            } else {
                let mut rows = build_rows(channel, &self.config);
                if more {
                    rows.insert(0, Row::LoadMore { loading });
                }
                let rows = Rc::new(rows);
                let cache = MessageRowsCache {
                    channel_id: key.0,
                    revision: key.1,
                    status: key.2,
                    more: key.3,
                    loading: key.4,
                    rows: rows.clone(),
                };
                (rows, Some(cache))
            };
            let nick_width = if self.config.nick_column && !self.narrow {
                nick_column_width(channel, self.config.font_size)
            } else {
                0.
            };
            let status_column = if nick_width > 0. { nick_width } else { 22. };
            let summary_indent =
                px(16. + if self.config.show_timestamps { 62. } else { 0. } + status_column + 8.);
            (rows, nick_width, summary_indent, cache_update)
        };
        if let Some(cache) = cache_update {
            self.message_rows_cache = Some(cache);
        }
        let keys = rows.iter().map(|row| self.list_row_key(row)).collect();
        self.sync_message_list(keys);
        if self.scroll_to_bottom_pending {
            self.messages_list.scroll_to(ListOffset {
                item_ix: self.messages_list.item_count(),
                offset_in_item: px(0.),
            });
            self.scroll_to_bottom_pending = false;
            self.messages_at_bottom = true;
        }

        self.trim_image_cache(window, cx);
        if let Some(message_id) = self.pending_focus_message.take() {
            if let Some(index) = self.message_row_index(rows.as_slice(), message_id) {
                self.messages_list.scroll_to_reveal_item(index);
                self.messages_at_bottom = false;
                self.pending_context = None;
            } else if let Some(context) = self.pending_context.take() {
                self.message_context = Some(context);
                window.focus(&self.root_focus);
            } else {
                self.toast("Message is not loaded.");
            }
        }

        let chrome = if self.narrow { 110. } else { 80. };
        let viewport_height = window.window_bounds().get_bounds().size.height;
        let max_list_height = (viewport_height - px(chrome)).max(px(120.));
        let list_height = if self.messages_available > px(0.) {
            self.messages_available.min(max_list_height)
        } else {
            (viewport_height - px(150.)).max(px(120.))
        };
        let entity = cx.entity();
        let item_rows = rows.clone();
        let item_entity = entity.clone();
        let bounds_weak = entity.downgrade();
        let selection_weak = entity.downgrade();
        div()
            .flex()
            .flex_1()
            .w_full()
            .min_h(px(0.))
            .relative()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.first()
                    && let Some(this) = bounds_weak.upgrade()
                {
                    this.update(cx, |this, _| this.messages_bounds = Some(*bounds));
                }
            })
            .id("messages")
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &gpui::MouseUpEvent, _, cx| {
                    if this.selecting_text {
                        this.selecting_text = false;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(move |event: &gpui::MouseMoveEvent, _, cx| {
                let Some(this) = selection_weak.upgrade() else {
                    return;
                };
                this.update(cx, |this, cx| {
                    if !this.selecting_text || event.pressed_button != Some(MouseButton::Left) {
                        return;
                    }
                    let Some(bounds) = this.messages_bounds else {
                        return;
                    };
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
                });
            })
            .child(
                list(self.messages_list.clone(), move |index, _, cx| {
                    let rows = item_rows.clone();
                    let entity = item_entity.clone();
                    entity.update(cx, |this, cx| {
                        this.render_list_row(index, &rows, nick_width, summary_indent, cx)
                    })
                })
                .w_full()
                .h(list_height)
                .min_h(px(0.))
                .line_height(relative(self.config.line_height.max(1.3)))
                .text_size(px(self.config.font_size))
                .font_family(self.config.font_family.clone()),
            )
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
        #[cfg(feature = "media-video")]
        let row_id = message.id;
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
            .and_then(|selection| selection.range_for(channel, message.id, &text))
        {
            runs = select_runs(runs, range, theme.accent_dim);
        }
        let selection_text = text.clone();

        let weak = cx.entity().downgrade();
        #[cfg(feature = "media-video")]
        let inline_message_id = message.id;
        let targets = Rc::new(targets);
        let click = {
            let weak = weak.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                let Some(target) = targets.get(index).cloned() else {
                    return;
                };
                match target {
                    ClickTarget::Url(url) => {
                        let _ = weak.update(cx, |this, cx| {
                            #[cfg(feature = "media-video")]
                            if let Some(kind) = super::media::media_kind(&url) {
                                this.play_inline_media(inline_message_id, url.clone(), kind, cx);
                                return;
                            }
                            this.play_media_or_open(url, cx);
                        });
                    }
                    ClickTarget::Nick(nick) => {
                        let _ = weak.update(cx, |this, cx| this.insert_nick(&nick, cx));
                    }
                }
            }
        };

        let message_id = message.id;
        let channel_id = channel.chan.id;
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
                #[cfg(feature = "media-video")]
                let mut inline_active = false;
                #[cfg(not(feature = "media-video"))]
                let inline_active = false;
                #[cfg(feature = "media-video")]
                {
                    let kind = if video {
                        super::media::MediaKind::Video
                    } else {
                        super::media::MediaKind::Audio
                    };
                    if let Some(inline) = self.render_inline_media(row_id, &play_url, cx) {
                        card = card.child(inline);
                        inline_active = true;
                    } else {
                        card = card.on_click(cx.listener(move |this, _, _, cx| {
                            this.play_inline_media(row_id, play_url.clone(), kind, cx);
                        }));
                    }
                }
                #[cfg(not(feature = "media-video"))]
                {
                    card = card.on_click(cx.listener(move |this, _, _, cx| {
                        this.play_media_or_open(play_url.clone(), cx);
                    }));
                }
                if !inline_active {
                    card = card.child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.accent)
                            .child(if video { "Play video" } else { "Play audio" }),
                    );
                }
                if !inline_active && let Some(thumb) = thumb {
                    self.touch_image(&thumb);
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
                self.touch_image(&thumb);
                let lightbox_url = thumb.clone();
                card = card
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_lightbox(lightbox_url.clone(), cx);
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
            self.touch_image(url);
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
                        this.open_lightbox(lightbox_url.clone(), cx);
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

        #[cfg(feature = "media-video")]
        let mut media_cards = div().flex().flex_col().gap_2();
        #[cfg(not(feature = "media-video"))]
        let media_cards = div().flex().flex_col().gap_2();
        #[cfg(feature = "media-video")]
        {
            for (index, link) in format::find_links(&message.text).into_iter().enumerate() {
                let url = self.absolute_media_url(&link.url);
                let Some(kind) = super::media::media_kind(&url) else {
                    continue;
                };
                let file = url.rsplit('/').next().unwrap_or(url.as_str()).to_string();
                let label = match kind {
                    super::media::MediaKind::Video => "Play video",
                    super::media::MediaKind::Audio => "Play audio",
                };
                let badge = match kind {
                    super::media::MediaKind::Video => "VIDEO",
                    super::media::MediaKind::Audio => "AUDIO",
                };
                let mut card = div()
                    .id(ElementId::Name(
                        format!("media-link-{row_id}-{index}").into(),
                    ))
                    .w_full()
                    .max_w(px(460.))
                    .rounded(px(8.))
                    .bg(theme.panel)
                    .border_1()
                    .border_color(theme.border)
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_1()
                                    .py(px(1.))
                                    .rounded(px(3.))
                                    .bg(theme.sidebar)
                                    .text_size(px(9.))
                                    .text_color(theme.muted)
                                    .child(badge),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .truncate()
                                    .text_size(px(11.))
                                    .text_color(theme.muted)
                                    .child(file),
                            ),
                    );
                if let Some(inline) = self.render_inline_media(row_id, &url, cx) {
                    card = card.child(inline);
                } else {
                    let play_url = url.clone();
                    card = card
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.play_inline_media(row_id, play_url.clone(), kind, cx);
                        }))
                        .cursor_pointer();
                    if kind == super::media::MediaKind::Video
                        && let Some(preview) = self.media_preview(&url, cx)
                    {
                        let width = if self.narrow { px(300.) } else { px(440.) };
                        let height = width * 0.5625;
                        card = card.child(
                            div()
                                .relative()
                                .w_full()
                                .h(height)
                                .rounded(px(5.))
                                .overflow_hidden()
                                .child(
                                    gpui_video_player::video(preview)
                                        .id(ElementId::Name(
                                            format!("media-preview-{row_id}-{index}").into(),
                                        ))
                                        .buffer_capacity(0)
                                        .size(width, height),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .bottom_2()
                                        .right_2()
                                        .px_2()
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(theme.overlay)
                                        .text_size(px(11.))
                                        .text_color(theme.text)
                                        .child("Play"),
                                ),
                        );
                    } else if kind == super::media::MediaKind::Audio {
                        card = card.child(
                            div()
                                .h(px(44.))
                                .w_full()
                                .rounded(px(5.))
                                .bg(theme.sidebar)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(18.))
                                .text_color(theme.accent)
                                .child("♪"),
                        );
                    }
                    card = card.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.accent)
                            .child(label),
                    );
                }
                media_cards = media_cards.child(card);
            }
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
                    .line_height(relative(self.config.line_height.max(1.3)))
                    .child(text_element)
                    .child(inline)
                    .child(previews)
                    .child(media_cards),
            );

        if let Some(reply_button) = reply_button {
            content = content.child(reply_button);
        }

        let context_message = message.clone();
        let focused = self.focused_message == Some(message.id);
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
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.message_menu = Some(MessageMenu {
                        position: event.position,
                        channel_id,
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

pub(super) fn date_label(date: chrono::NaiveDate) -> String {
    let today = Local::now().date_naive();
    if date == today {
        "Today".into()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".into()
    } else {
        date.format("%e %B %Y").to_string().trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_range_handles_unicode_and_boundaries() {
        let text = "hello wörld_test!";
        assert_eq!(word_range(text, 2), 0..5);
        assert_eq!(word_range(text, 7), 6..17);
        assert_eq!(word_range(text, 17), 6..17);
        assert_eq!(word_range(text, 18), 18..18);
    }

    #[test]
    fn word_range_includes_irc_nick_characters() {
        let text = "prefix [nick-name] suffix";
        assert_eq!(word_range(text, 9), 8..17);
    }
}
