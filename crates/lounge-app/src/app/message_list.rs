use super::message_text::date_label;
use super::*;

pub(super) fn build_rows(channel: &ChannelState, config: &AppConfig) -> Vec<Row> {
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
                messages: group.to_vec(),
            });
        } else if group.len() == 1
            && !(config.status_messages == StatusMessages::Hidden && group[0].kind.is_condensable())
        {
            rows.push(Row::Message(Box::new(group[0].clone())));
        }

        index = condensed_end;
    }

    if config.status_messages == StatusMessages::Hidden {
        rows.retain(|row| !matches!(row, Row::Message(message) if message.kind.is_condensable()));
    }

    rows
}

#[derive(Clone)]
pub(super) struct MessageRowsCache {
    pub channel_id: i64,
    pub revision: u64,
    pub status: StatusMessages,
    pub more: bool,
    pub loading: bool,
    pub rows: Rc<Vec<Row>>,
}

#[derive(Clone, Debug)]
pub(super) enum Row {
    LoadMore { loading: bool },
    Date(String),
    Unread,
    Message(Box<Message>),
    Condensed { id: i64, messages: Vec<Message> },
}

pub(super) fn nick_column_width(channel: &ChannelState, font_size: f32) -> f32 {
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

impl LoungeApp {
    pub(super) fn list_row_key(&self, row: &Row) -> String {
        match row {
            Row::LoadMore { loading } => format!("more:{loading}"),
            Row::Date(label) => format!("date:{label}"),
            Row::Unread => "unread".to_string(),
            Row::Message(message) => format!("message:{message:?}"),
            Row::Condensed { id, messages } => format!(
                "condensed:{id}:{}:{messages:?}",
                self.expanded_condensed.contains(id)
            ),
        }
    }

    pub(super) fn sync_message_list(&mut self, keys: Vec<String>) {
        let old_count = self.messages_list.item_count();
        if old_count == 0 {
            self.messages_list.reset(keys.len());
            self.messages_list_keys = keys;
            return;
        }
        if old_count == keys.len() && self.messages_list_keys == keys {
            return;
        }
        let old_top = self.messages_list.logical_scroll_top();
        let old_top_key = self.messages_list_keys.get(old_top.item_ix).cloned();
        let common = old_count.min(keys.len());
        for (index, key) in keys.iter().take(common).enumerate() {
            if self.messages_list_keys.get(index) != Some(key) {
                self.messages_list.splice(index..index + 1, 1);
            }
        }
        if keys.len() > old_count {
            self.messages_list
                .splice(old_count..old_count, keys.len() - old_count);
        } else if keys.len() < old_count {
            self.messages_list.splice(keys.len()..old_count, 0);
        }
        if let Some(old_top_key) = old_top_key
            && let Some(new_index) = keys.iter().position(|key| key == &old_top_key)
            && new_index != old_top.item_ix
        {
            self.messages_list.scroll_to(ListOffset {
                item_ix: new_index,
                offset_in_item: old_top.offset_in_item,
            });
        }
        self.messages_list_keys = keys;
    }

    pub(super) fn message_row_index(&self, rows: &[Row], message_id: i64) -> Option<usize> {
        message_row_index(rows, message_id)
    }

    pub(super) fn render_list_row(
        &mut self,
        index: usize,
        rows: &[Row],
        nick_width: f32,
        summary_indent: Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(row) = rows.get(index) else {
            return div().into_any_element();
        };
        let theme = self.theme;
        let Some(active) = self.state.active else {
            return div().into_any_element();
        };
        let Some((network, channel)) = self.state.find_channel(active) else {
            return div().into_any_element();
        };
        match row {
            Row::LoadMore { loading } => {
                let label = if *loading {
                    "Loading older messages..."
                } else {
                    "Show older messages"
                };
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .py_2()
                    .child(
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
                    )
                    .into_any_element()
            }
            Row::Date(label) => div()
                .px_4()
                .py_2()
                .flex()
                .items_center()
                .gap_3()
                .child(div().flex_1().h(px(1.)).bg(theme.border))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted)
                        .child(label.clone()),
                )
                .child(div().flex_1().h(px(1.)).bg(theme.border))
                .into_any_element(),
            Row::Unread => div()
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
                .child(div().flex_1().h(px(1.)).bg(theme.accent))
                .into_any_element(),
            Row::Message(message) => {
                self.render_message_row(network, channel, message, nick_width, cx)
            }
            Row::Condensed { id, messages } => {
                let id = *id;
                let summary = condensed_summary(messages);
                let expanded = self.expanded_condensed.contains(&id);
                let mut container = div()
                    .id(ElementId::Name(format!("cond-{id}").into()))
                    .flex()
                    .flex_col()
                    .w_full();
                container = container.child(
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
                        container = container.child(
                            self.render_message_row(network, channel, message, nick_width, cx),
                        );
                    }
                }
                container.into_any_element()
            }
        }
    }
}

fn message_row_index(rows: &[Row], message_id: i64) -> Option<usize> {
    rows.iter().position(|row| match row {
        Row::Message(message) => message.id == message_id,
        Row::Condensed { messages, .. } => messages.iter().any(|message| message.id == message_id),
        _ => false,
    })
}

fn condensed_summary(messages: &[Message]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn status_channel(kind: MessageType, count: i64) -> ChannelState {
        ChannelState {
            chan: lounge_core::proto::Channel {
                kind: lounge_core::proto::ChanType::Channel,
                messages: (1..=count)
                    .map(|id| Message {
                        id,
                        kind,
                        text: format!("status {id}"),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn condensed_status_messages_become_one_owned_row() {
        let channel = status_channel(MessageType::Join, 3);
        let config = AppConfig::default();
        let rows = build_rows(&channel, &config);
        assert_eq!(rows.len(), 1);
        assert!(matches!(
            &rows[0],
            Row::Condensed { messages, .. } if messages.len() == 3
        ));
    }

    #[test]
    fn message_rows_map_jump_targets() {
        let channel = status_channel(MessageType::Join, 3);
        let config = AppConfig::default();
        let rows = build_rows(&channel, &config);
        assert_eq!(message_row_index(&rows, 2), Some(0));
        assert_eq!(message_row_index(&rows, 99), None);
    }

    #[test]
    fn hidden_status_messages_are_removed_from_rows() {
        let channel = status_channel(MessageType::Join, 3);
        let config = AppConfig {
            status_messages: StatusMessages::Hidden,
            ..Default::default()
        };
        let rows = build_rows(&channel, &config);
        assert!(rows.is_empty());
    }
}
