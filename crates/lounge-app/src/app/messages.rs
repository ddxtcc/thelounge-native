use super::message_text::{build_message_text, user_mode_prefix_in_message};
use super::*;

impl LoungeApp {
    pub(super) fn render_messages(
        &mut self,
        _window: &mut Window,
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
        let Some((network, channel)) = self.state.find_channel(active) else {
            return div().flex_1().into_any_element();
        };
        if let Some(special) = channel.chan.special.clone() {
            return self.render_special(network, channel, &special, cx);
        }

        let rows = build_rows(channel, &self.config);
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
                            .text_size(px(11.))
                            .text_color(theme.muted)
                            .child(label),
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
                    content = content.child(self.render_message_row(network, channel, message, cx));
                }
                Row::Condensed { id, messages } => {
                    let summary = condensed_summary(&messages);
                    let expanded = self.expanded_condensed.contains(&id);
                    content = content.child(
                        div()
                            .id(ElementId::Name(format!("cond-{id}").into()))
                            .px_4()
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
                            .child(format!("{} {}", if expanded { "v" } else { ">" }, summary)),
                    );
                    if expanded {
                        for message in messages {
                            content = content
                                .child(self.render_message_row(network, channel, message, cx));
                        }
                    }
                }
            }
        }

        div()
            .id("messages")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.messages_scroll)
            .child(content)
            .into_any_element()
    }

    pub(super) fn render_message_row(
        &self,
        network: &NetworkState,
        channel: &ChannelState,
        message: &Message,
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
        let body = build_message_text(
            message,
            network,
            channel,
            &self.config,
            &theme,
            self.theme.dark,
        );
        let (text, runs, ranges, targets) = body;

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

        let text_element = if ranges.is_empty() {
            gpui::StyledText::new(text)
                .with_runs(runs)
                .into_any_element()
        } else {
            InteractiveText::new(
                ElementId::Name(format!("msg-{}", message.id).into()),
                StyledText::new(text).with_runs(runs),
            )
            .on_click(ranges, click)
            .into_any_element()
        };

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
                let candidate = preview
                    .thumb_actual_url
                    .as_deref()
                    .filter(|url| !url.is_empty())
                    .or_else(|| Some(preview.thumb.as_str()).filter(|url| !url.is_empty()))
                    .or_else(|| preview.media.as_deref().filter(|url| !url.is_empty()))
                    .unwrap_or(&preview.link);
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
                .on_click(cx.listener(move |_, _, _, _| {
                    let _ = open::that(url.clone());
                }))
                .flex()
                .flex_col()
                .gap_1()
                .child(div().truncate().child(label));
            if let Some(thumb) = thumb {
                card = card.child(
                    gpui::img(thumb)
                        .max_w(px(220.))
                        .max_h(px(140.))
                        .rounded(px(4.)),
                );
            }
            previews = previews.child(card);
        }

        let mut inline = div().flex().flex_col().gap_2();
        for (index, url) in inline_images.iter().enumerate() {
            let open_url = url.clone();
            inline = inline.child(
                div()
                    .id(ElementId::Name(
                        format!("inline-image-{}-{index}", message.id).into(),
                    ))
                    .max_w(px(460.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .on_click(cx.listener(move |_, _, _, _| {
                        let _ = open::that(open_url.clone());
                    }))
                    .child(
                        gpui::img(url.clone())
                            .max_w(px(460.))
                            .max_h(px(360.))
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

        let mut row = div()
            .id(ElementId::Name(format!("row-{}", message.id).into()))
            .flex()
            .w_full()
            .items_start()
            .gap_2()
            .px_4()
            .py(px(self.config.message_spacing))
            .when(highlight, |this| this.bg(theme.highlight_bg))
            .when(is_self, |this| this.opacity(0.85));

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
        if self.config.nick_column {
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
                rgb(format::nick_color(&nick)).into()
            } else {
                theme.text
            };
            let insert = nick.clone();
            let mut column = div()
                .id(ElementId::Name(format!("nickcol-{}", message.id).into()))
                .w(px(132.))
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

    pub(super) fn render_special(
        &self,
        network: &NetworkState,
        channel: &ChannelState,
        special: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let data = channel.chan.data.clone();
        let rows: Vec<serde_json::Value> = data
            .as_ref()
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let status_text = data
            .as_ref()
            .and_then(|value| value.get("text"))
            .and_then(|value| value.as_str())
            .map(str::to_string);

        let mut headers: Vec<(&str, Option<gpui::Pixels>)> = Vec::new();
        match special {
            "list_bans" => {
                headers = vec![
                    ("Banned", None),
                    ("Banned by", Some(px(180.))),
                    ("Banned at", Some(px(210.))),
                ];
            }
            "list_invites" => {
                headers = vec![
                    ("Invited", None),
                    ("Invited by", Some(px(180.))),
                    ("Invited at", Some(px(210.))),
                ];
            }
            "list_ignored" => {
                headers = vec![("Hostmask", None), ("Ignored at", Some(px(230.)))];
            }
            "list_channels" => {
                headers = vec![
                    ("Channel", Some(px(240.))),
                    ("Users", Some(px(90.))),
                    ("Topic", None),
                ];
            }
            _ => {}
        }

        let mut table = div().flex().flex_col().w_full();
        if !headers.is_empty() {
            let mut header_row = div()
                .flex()
                .gap_4()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(11.))
                .text_color(theme.muted);
            for (label, width) in &headers {
                header_row = header_row.child(
                    div()
                        .when_some(*width, |this, width| this.w(width))
                        .when(width.is_none(), |this| this.flex_1())
                        .child(label.to_string()),
                );
            }
            table = table.child(header_row);
        }

        if let Some(text) = status_text {
            table = table.child(div().p_4().text_color(theme.muted).child(text));
        } else if rows.is_empty() && !headers.is_empty() {
            table = table.child(div().p_4().text_color(theme.muted).child("No entries."));
        } else {
            for row in rows {
                let mut cells = div()
                    .flex()
                    .gap_4()
                    .px_4()
                    .py(px(5.))
                    .border_b_1()
                    .border_color(theme.border)
                    .items_start();
                match special {
                    "list_bans" => {
                        cells = cells
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(json_string(&row, "hostmask")),
                            )
                            .child(div().w(px(180.)).child(json_string(&row, "banned_by")))
                            .child(
                                div()
                                    .w(px(210.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("banned_at"))),
                            );
                    }
                    "list_invites" => {
                        cells = cells
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(json_string(&row, "hostmask")),
                            )
                            .child(div().w(px(180.)).child(json_string(&row, "invited_by")))
                            .child(
                                div()
                                    .w(px(210.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("invited_at"))),
                            );
                    }
                    "list_ignored" => {
                        cells = cells
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(json_string(&row, "hostmask")),
                            )
                            .child(
                                div()
                                    .w(px(230.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("when"))),
                            );
                    }
                    "list_channels" => {
                        let name = json_string(&row, "channel");
                        let join_name = name.clone();
                        let exists = network
                            .channels
                            .iter()
                            .find(|channel| channel.chan.name.eq_ignore_ascii_case(&name))
                            .map(|channel| channel.chan.id);
                        let lobby = network.channels.first().map(|channel| channel.chan.id);
                        cells = cells
                            .child(
                                div()
                                    .w(px(240.))
                                    .id(ElementId::Name(format!("special-join-{name}").into()))
                                    .text_color(theme.accent)
                                    .cursor_pointer()
                                    .hover(|style| style.underline())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if let Some(id) = exists {
                                            this.switch_channel(id, window, cx);
                                        } else if let (Some(conn), Some(lobby)) =
                                            (&this.conn, lobby)
                                        {
                                            conn.input(lobby, format!("/join {join_name}"), None);
                                        }
                                    }))
                                    .child(name),
                            )
                            .child(
                                div().w(px(90.)).text_color(theme.muted).child(
                                    row.get("num_users")
                                        .map(|value| value.to_string())
                                        .unwrap_or_default(),
                                ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .text_color(theme.muted)
                                    .child(json_string(&row, "topic")),
                            );
                    }
                    _ => {
                        cells = cells.child(row.to_string());
                    }
                }
                table = table.child(cells);
            }
        }

        div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .text_size(px(12.))
                    .text_color(theme.muted)
                    .child(format!("{} - read-only", channel.chan.name)),
            )
            .child(
                div()
                    .id("special-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .child(table),
            )
            .into_any_element()
    }
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .map(|value| match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn special_time(value: Option<&serde_json::Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let datetime = if let Some(millis) = value.as_i64() {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)
    } else if let Some(text) = value.as_str() {
        chrono::DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|datetime| datetime.with_timezone(&chrono::Utc))
    } else {
        None
    };
    datetime
        .map(|datetime| {
            datetime
                .with_timezone(&Local)
                .format("%d %B %Y, %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
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

fn date_label(date: chrono::NaiveDate) -> String {
    let today = Local::now().date_naive();
    if date == today {
        "Today".into()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".into()
    } else {
        date.format("%A, %B %e, %Y").to_string()
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
        plural(joins, "has joined", "have joined"),
        plural(parts, "has left", "have left"),
        plural(nicks, "has changed nickname", "have changed nicknames"),
        plural(kicks, "was kicked", "were kicked"),
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
        if modes == 1 {
            "1 mode was set".into()
        } else if modes > 1 {
            format!("{modes} modes were set")
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
    ] {
        if !value.is_empty() {
            strings.push(value);
        }
    }
    if strings.is_empty() {
        format!("{} status messages", messages.len())
    } else {
        strings.join(", ")
    }
}
